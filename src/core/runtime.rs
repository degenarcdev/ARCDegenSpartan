use rand::Rng;
use tokio::time::{sleep, Duration};

use crate::{
    core::agent::Agent,
    memory::MemoryStore,
    providers::{ai16z_twitter::Ai16zTwitter, discord::Discord, twitter::Twitter},
};

pub enum TwitterType {
    ApiKeys(Twitter),
    Ai16zTwitter(Ai16zTwitter),
}

impl TwitterType {
    pub async fn tweet(&self, text: &str) -> Result<(), anyhow::Error> {
        match self {
            TwitterType::ApiKeys(twitter) => {
                // Call the tweet method for Twitter API
                twitter.tweet(text.to_string()).await
            }
            TwitterType::Ai16zTwitter(ai6z_twitter) => {
                // Call the tweet method for Ai6zTwitter
                ai6z_twitter.tweet(text.to_string()).await
            }
        }
    }
}

pub struct Runtime {
    openai_api_key: String,
    twitter: TwitterType,
    discord: Discord,
    agents: Vec<Agent>,
    memory: Vec<String>,
}

impl Runtime {
    pub fn new(
        openai_api_key: &str,
        discord_webhook_url: &str,
        twitter_consumer_key: Option<&str>,
        twitter_consumer_secret: Option<&str>,
        twitter_access_token: Option<&str>,
        twitter_access_token_secret: Option<&str>,
        twitter_username: Option<&str>,
        twitter_password: Option<&str>,
    ) -> Self {
        let twitter = match (twitter_username, twitter_password) {
            (Some(username), Some(password)) => {
                // If both username and password are provided, prioritize Ai6zTwitter
                TwitterType::Ai16zTwitter(Ai16zTwitter::new(username, password))
            }
            (_, _) => {
                // Otherwise, fall back to Twitter API keys if available
                match (
                    twitter_consumer_key,
                    twitter_consumer_secret,
                    twitter_access_token,
                    twitter_access_token_secret,
                ) {
                    (
                        Some(consumer_key),
                        Some(consumer_secret),
                        Some(access_token),
                        Some(access_token_secret),
                    ) => TwitterType::ApiKeys(Twitter::new(
                        consumer_key,
                        consumer_secret,
                        access_token,
                        access_token_secret,
                    )),
                    _ => panic!("You must provide either Twitter username/password or API keys."),
                }
            }
        };
        let discord = Discord::new(discord_webhook_url);

        let agents = Vec::new();
        let memory: Vec<String> = MemoryStore::load_memory().unwrap_or_else(|_| Vec::new());

        Runtime {
            discord,
            memory,
            openai_api_key: openai_api_key.to_string(),
            agents,
            twitter,
        }
    }

    pub fn add_agent(&mut self, prompt: &str) {
        let agent = Agent::new(&self.openai_api_key, prompt);
        self.agents.push(agent);
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        if self.agents.is_empty() {
            return Err(anyhow::anyhow!("No agents available")).map_err(Into::into);
        }

        let mut rng = rand::thread_rng();
        let selected_agent = &self.agents[rng.gen_range(0..self.agents.len())];
        let response = selected_agent.prompt("tweet").await?;

        match MemoryStore::add_to_memory(&mut self.memory, &response) {
            Ok(_) => println!("Response saved to memory."),
            Err(e) => eprintln!("Failed to save response to memory: {}", e),
        }

        println!("AI Response: {}", response);
        self.discord.send_channel_message(&response.clone()).await;
        self.twitter.tweet(&response).await?;
        Ok(())
    }

    pub async fn run_periodically(&mut self) -> Result<(), anyhow::Error> {
        let mut rng = rand::thread_rng();

        loop {
            let random_sleep_duration = rng.gen_range(300..=1800);

            sleep(Duration::from_secs(random_sleep_duration)).await;

            if let Err(e) = self.run().await {
                eprintln!("Error running process: {}", e);
            }
        }
    }
}
