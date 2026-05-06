use anyhow::anyhow;

pub fn validate_token(token: Option<&str>) -> anyhow::Result<&str> {
    let Some(token) = token else {
        return Err(anyhow!("no token was provided"));
    };

    match serenity::utils::validate_token(token) {
        Ok(()) => Ok(token),
        Err(_) => Err(anyhow!("an invalid token was provided")),
    }
}
