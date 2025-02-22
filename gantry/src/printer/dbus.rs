use std::collections::HashMap;
use std::sync::Arc;

use gantry_api::*;

use super::Instance;

pub struct DBusInstance {
    pub inner: Arc<Instance>,
}

#[zbus::interface(name = "org.gantry.Printer")]
impl DBusInstance {
    /////////////////////////////////////////////
    ///////////      Authentication    //////////
    /////////////////////////////////////////////

    /// login to the printer
    pub async fn login(&self, pwd: &str) -> PrinterResult<PrinterLogin> {
        self.inner.login(pwd).await
    }

    /// logout from the printer
    pub async fn logout(&self, token: &str) -> PrinterResult<()> {
        self.inner.logout(token).await
    }

    /// reset password
    pub async fn reset_password(&self, token: &str, new_password: &str) -> PrinterResult<()> {
        self.inner.reset_password(token, new_password).await
    }

    /// refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> PrinterResult<PrinterLogin> {
        self.inner.refresh_token(refresh_token).await
    }

    /////////////////////////////////////////////
    ///////////         Status        ///////////
    /////////////////////////////////////////////

    /// get printer info
    pub async fn get_info(&self, token: &str) -> PrinterResult<PrinterInfo> {
        // check for token only
        if let Err(err) = self.inner.validate_token(token) {
            return PrinterResult::err(err);
        }

        return self.inner.get_info().await;
    }

    /// emergency stop
    pub async fn emergency_stop(&self, token: &str) -> PrinterResult<()> {
        // check for token only
        if let Err(err) = self.inner.validate_token(token) {
            return PrinterResult::err(err);
        }

        self.inner.emergency_stop();

        return PrinterResult::ok(());
    }

    /// restart gantry
    pub async fn restart(&self, token: &str) -> PrinterResult<()> {
        // check for token only
        if let Err(err) = self.inner.validate_token(token) {
            return PrinterResult::err(err);
        }

        self.inner.restart().await;

        return PrinterResult::ok(());
    }

    /// list objects loaded
    pub async fn list_objects(&self, token: &str) -> PrinterResult<HashMap<String, String>> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.list_objects().await;
    }

    /// query endstop status
    pub async fn query_endstops(&self, token: &str) -> PrinterResult<PrinterEndstopStatus> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.query_endstops().await;
    }

    /////////////////////////////////////////////
    ///////////       Extensions      ///////////
    /////////////////////////////////////////////

    /// list extensions loaded
    pub async fn list_extensions(
        &self,
        token: &str,
    ) -> PrinterResult<HashMap<String, PrinterExtension>> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.list_extensions().await;
    }

    /// install an extension
    pub async fn install_extension(&self, token: &str, repo: String) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.install_extension(repo).await;
    }

    /// remove an extension
    pub async fn remove_extension(&self, token: &str, name: String) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.remove_extension(name).await;
    }

    /// download extension config
    pub async fn download_extension_config(
        &self,
        token: &str,
        name: &str,
    ) -> PrinterResult<String> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.download_extension_config(name).await;
    }

    /// upload extension config
    pub async fn upload_extension_config(
        &self,
        token: &str,
        name: &str,
        config: String,
    ) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.upload_extension_config(name, config).await;
    }

    /////////////////////////////////////////////
    ///////////       Gcode API       ///////////
    /////////////////////////////////////////////

    /// execute a gcode script
    pub async fn run_gcode(&self, token: &str, script: String) -> PrinterResult<()> {
        // check token
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        // run gcode script
        return self.inner.run_gcode(script).await;
    }

    /// Retrieves a list of registered GCode Command Descriptions.
    pub async fn get_gcode_help(&self, token: &str) -> PrinterResult<HashMap<String, String>> {
        // check token
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.get_gcode_help().await;
    }

    /////////////////////////////////////////////
    ///////////       Print job       ///////////
    /////////////////////////////////////////////

    /// start a print job
    pub async fn start_print_job(&self, token: &str, filename: &str) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.start_print_job(filename).await;
    }
    /// pause the print job
    pub async fn pause_print_job(&self, token: &str) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.pause_print_job().await;
    }
    /// resume the print job
    pub async fn resume_print_job(&self, token: &str) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        return self.inner.resume_print_job().await;
    }
    /// cancel the print job
    pub async fn cancel_print_job(&self, token: &str) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.cancel_print_job().await;

        return PrinterResult::ok(());
    }

    /// queue print job to run after current print job is finished
    pub async fn queue_print_job(
        &self,
        token: &str,
        filename: &str,
    ) -> PrinterResult<PrinterQueuePrintJob> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.queue_print_job(filename).await
    }

    //// delete a print job in queue
    pub async fn delete_queue_print_job(&self, token: &str, id: u64) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.delete_queue_print_job(id).await
    }

    /////////////////////////////////////////////
    ///////////      Gcode files      ///////////
    /////////////////////////////////////////////

    /// list avaliable gcode files
    pub async fn list_files(&self, token: &str) -> PrinterResult<Vec<PrinterGcodeFile>> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.list_files().await
    }
    /// get metadata for a specified gcode file
    pub async fn get_file_metadata(&self, token: &str, filename: &str) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.get_file_metadata(filename).await
    }
    /// Initiate a metadata scan for a selected file. If the file has already been scanned the endpoint will force a re-scan.
    pub async fn scan_file_metadata(&self, token: &str, filename: &str) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.scan_file_metadata(filename).await
    }
    /// upload a gcode file
    pub async fn upload_file(
        &self,
        token: &str,
        filename: &str,
        filedata: String,
    ) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.upload_file(filename, filedata).await
    }
    /// download a gcode file
    pub async fn download_file(&self, token: &str, filename: &str) -> PrinterResult<String> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.download_file(filename).await
    }
    /// download the printer config
    pub async fn download_printer_config(&self, token: &str) -> PrinterResult<String> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.download_printer_config().await
    }
    /// upload the printer config
    pub async fn upload_printer_config(&self, token: &str, config: String) -> PrinterResult<()> {
        if let Some(err) = self.inner.validate_token_state(token).await {
            return PrinterResult::err(err);
        }

        self.inner.upload_printer_config(config).await
    }
}
