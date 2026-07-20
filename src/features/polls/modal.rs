#[derive(Debug, poise::Modal)]
pub struct NewPollModal {
    #[name = "Poll Title"]
    pub title: String,

    #[name = "Option 1 (Name : Weight)"]
    #[placeholder = "Yes : 1.0"]
    pub opt_1: String,

    #[name = "Option 2 (Name : Weight)"]
    #[placeholder = "No : 1.0"]
    pub opt_2: String,

    #[name = "Option 3 (Name : Weight - Optional)"]
    #[placeholder = "HardNo : 1.5"]
    pub opt_3: Option<String>,

    #[name = "Options 4-8 (Name:Weight, Name:Weight...)"]
    #[placeholder = "Maybe:1, Pizza:1, Tacos:1"]
    pub opt_bulk: Option<String>,
}

pub fn parse_weight(input: Option<String>) -> f64 {
    input
        .and_then(|s| s.replace(',', ".").parse::<f64>().ok())
        .unwrap_or(1.0)
}
