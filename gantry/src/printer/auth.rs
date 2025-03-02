pub struct Auth {
    printer_uuid: u128,
}

impl Auth {
    pub fn acquire(printer_uuid: u128) -> Self {
        Self { printer_uuid }
    }

    /// login printer, returns jwt token and refresh token
    pub fn login(&self, password: &str) -> Option<(String, String)> {
        crate::global_auth::login(itoa::Buffer::new().format(self.printer_uuid), password)
    }

    /// logout from printer, token would be invalidated
    pub fn logout(&self, token: &str) -> bool {
        crate::global_auth::logout(token)
    }

    /// returns (is_valid, is_timeout)
    pub fn validate_token(&self, token: &str) -> (bool, bool) {
        crate::global_auth::validate_token(token)
    }

    pub fn reset_password(&self, token: &str, password: &str) -> bool {
        crate::global_auth::reset_password(token, password)
    }

    pub fn refresh_token(&self, refresh_token: &str) -> Option<(String, String)> {
        crate::global_auth::refresh_token(refresh_token)
    }
}
