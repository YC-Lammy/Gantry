

/// login a user, returns bearer and refresh token
pub fn login(username: &str, password: &str) -> Option<(String, String)>{
    todo!()
}

pub fn logout(token: &str) -> bool{
    todo!()
}

pub fn reset_password(token: &str, password: &str) -> bool{
    todo!()
}

/// refresh bearer token using refresh token
pub fn refresh_token(refresh_token: &str) -> Option<(String, String)>{
    todo!()
}

pub fn validate_token(token: &str) -> (bool, bool){
    todo!()
}