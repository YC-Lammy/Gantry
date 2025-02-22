pub struct Auth {}

impl Auth {
    pub fn acquire(printer_uuid: u128) -> Self {
        todo!()
    }

    /// login printer, returns jwt token and refresh token
    pub fn login(&self, pwd: &str) -> Option<(String, String)> {
        todo!()
    }

    /// logout from printer, token would be invalidated
    pub fn logout(&self, token: &str) -> bool {
        todo!()
    }

    /// returns (is_valid, is_timeout)
    pub fn validate_token(&self, token: &str) -> (bool, bool) {
        todo!()
    }

    pub fn reset_password(&self, token: &str, password: &str) -> bool {
        todo!()
    }

    pub fn refresh_token(&self, refresh_token: &str) -> Option<(String, String)> {
        todo!()
    }
}
