
use std::sync::Arc;
use std::collections::HashMap;

use gantry_api::*;

use super::Instance;

pub struct DBusInstance{
    pub inner: Arc<Instance>
}

#[zbus::interface(name = "org.gantry.Printer")]
impl DBusInstance {
    /////////////////////////////////////////////
    ///////////      Authentication    //////////
    /////////////////////////////////////////////

    /// login to the printer
    pub async fn login(&self, pwd: &str) -> PrinterResult<PrinterLogin> {
        match self.inner.auth.login(pwd) {
            Some((token, refresh_token)) => PrinterResult::ok(PrinterLogin {
                token,
                refresh_token,
            }),
            None => PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthFailed,
                message: String::new(),
            }),
        }
    }

    /// logout from the printer
    pub async fn logout(&self, token: &str) -> PrinterResult<()> {
        match self.inner.auth.logout(token) {
            true => PrinterResult::ok(()),
            false => PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenInvalid,
                message: String::new(),
            }),
        }
    }

    /// reset password
    pub async fn reset_password(&self, token: &str, new_password: &str) -> PrinterResult<()> {
        if let Some(err) = self.inner.check_auth(token).await {
            return PrinterResult::err(err);
        }

        if !self.inner.auth.reset_password(new_password) {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthFailed,
                message: String::new(),
            });
        }

        return PrinterResult::ok(());
    }

    /// refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> PrinterResult<PrinterLogin> {
        match self.inner.auth.refresh_token(refresh_token) {
            Some((token, refresh_token)) => PrinterResult::ok(PrinterLogin {
                token,
                refresh_token,
            }),
            None => PrinterResult::err(PrinterError {
                code: PrinterErrorCode::RefreshTokenInvalid,
                message: String::new(),
            }),
        }
    }

    /////////////////////////////////////////////
    ///////////         Status        ///////////
    /////////////////////////////////////////////

    /// get printer info
    pub async fn get_info(&self, token: &str) -> PrinterResult<PrinterInfo> {
        // check for token
        let (is_valid, is_timeout) = self.inner.auth.validate_token(token);

        if is_timeout {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenTimeout,
                message: String::new(),
            });
        }

        if !is_valid {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenInvalid,
                message: String::new(),
            });
        }

        let error_state = self.inner.error_state().await;

        return PrinterResult::ok(PrinterInfo {
            state: self.inner.state(),
            error_state_code: error_state.code,
            error_state_message: error_state.message,
            printer_path: self.inner.printer_path.to_string_lossy().to_string(),
        });
    }

    /// emergency stop
    pub async fn emergency_stop(&self, token: &str) -> PrinterResult<()>{
        // check for token
        let (is_valid, is_timeout) = self.inner.auth.validate_token(token);

        if is_timeout {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenTimeout,
                message: String::new(),
            });
        }

        if !is_valid {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenInvalid,
                message: String::new(),
            });
        }

        // set state to shutdown
        self.inner.set_state(PrinterState::Shutdown);

        // block the current thread to stop ASAP
        tokio::task::block_in_place(|| {
            let mut printer = self.inner.printer.blocking_write();
            printer.emergency_stop();
        }
        );

        return PrinterResult::ok(())
    }

    /// restart gantry
    pub async fn restart(&self, token: &str) -> PrinterResult<()>{
        // check for token
        let (is_valid, is_timeout) = self.inner.auth.validate_token(token);

        if is_timeout {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenTimeout,
                message: String::new(),
            });
        }

        if !is_valid {
            return PrinterResult::err(PrinterError {
                code: PrinterErrorCode::AuthTokenInvalid,
                message: String::new(),
            });
        }

        self.inner.restart().await;

        return PrinterResult::ok(())
    }

    /// list objects loaded
    pub async fn list_objects(&self, token: &str) -> PrinterResult<HashMap<String, String>>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        todo!()
    }

    /// list extensions loaded
    pub async fn list_extensions(&self, token: &str) -> PrinterResult<HashMap<String, PrinterExtension>>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        todo!()
    }

    /// install an extension
    pub async fn install_extension(&self, token: &str, repo: String) -> PrinterResult<()>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        todo!()
    }

    /// remove an extension
    pub async fn remove_extension(&self, token: &str, name: String) -> PrinterResult<()>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        todo!()
    }

    /// query endstop status
    pub async fn query_endstops(&self, token: &str) -> PrinterResult<PrinterEndstopStatus>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        let printer = self.inner.printer.read().await;
        let status = printer.get_endstop_status().await;

        return PrinterResult::ok(PrinterEndstopStatus{
            x_triggered: status.0,
            y_triggered: status.1,
            z_triggered: status.2
        })
    }

    /////////////////////////////////////////////
    ///////////       Gcode API       ///////////
    /////////////////////////////////////////////
    
    /// execute a gcode script
    pub async fn run_gcode(&self, token: &str, script: String) -> PrinterResult<()>{
        // check token
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        // acquire write lock
        let mut printer = self.inner.printer.write().await;
        // run gcode script
        match printer.run_gcode(script).await{
            Ok(()) => PrinterResult::ok(()),
            Err(e) => PrinterResult::err(PrinterError { code: PrinterErrorCode::GcodeParseError, message: e })
        }
    }

    /// Retrieves a list of registered GCode Command Descriptions.
    pub async fn get_gcode_help(&self, token: &str) -> PrinterResult<HashMap<String, String>>{
        // check token
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        // acquire read lock
        let printer = self.inner.printer.read().await;

        return PrinterResult::ok(printer.get_gcode_help().await)
    }

    /////////////////////////////////////////////
    ///////////       Print job       ///////////
    /////////////////////////////////////////////
    
    /// start a print job
    pub async fn start_print_job(&self, token: &str, filename: String) -> PrinterResult<()>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        // acquire write lock
        let mut printer = self.inner.printer.write().await;

        if printer.start_print_job(filename).await{
            return PrinterResult::ok(())
        }

        return PrinterResult::err(PrinterError{
            code: PrinterErrorCode::PrintJobRunning,
            message: String::new()
        })
    }
    /// pause the print job
    pub async fn pause_print_job(&self, token: &str) -> PrinterResult<()>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        // acquire write lock
        let mut printer = self.inner.printer.write().await;

        printer.pause_print_job().await;

        return PrinterResult::ok(())
    }
    /// resume the print job
    pub async fn resume_print_job(&self, token: &str) -> PrinterResult<()>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        // acquire write lock
        let mut printer = self.inner.printer.write().await;

        printer.resume_print_job().await;

        return PrinterResult::ok(())
    }
    /// cancel the print job
    pub async fn cancel_print_job(&self, token: &str) -> PrinterResult<()>{
        if let Some(err) = self.inner.check_auth(token).await{
            return PrinterResult::err(err)
        }

        // acquire write lock
        let mut printer = self.inner.printer.write().await;

        printer.cancel_print_job().await;

        return PrinterResult::ok(())
    }
}