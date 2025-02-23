
export class PrinterError{
    is_http_error;
    error_code;
    message;

    constructor(is_http_error, error_code, message){
        this.is_http_error = is_http_error;
        this.error_code = error_code;
        this.message = message;
    }

    errorKine(){
        if (this.is_http_error){
            return "HTTP Error: " + this.error_code
        }

        return this.error_code
    }

    toString(){
        this.errorKine() + ": " + this.message
    }
}

export class Printer{
    #name;
    #url;
    #bearer;
    #refresh_token;

    constructor(name, url){
        this.#name = name;
        this.#url = url;
    }

    check_tokens(bearer, refresh_token){
        this.#bearer = bearer;
        try{
            this.get_info();
            this.#refresh_token = refresh_token;
            return true;
        } catch{
            this.#bearer = null;
            return false;
        }
    }

    is_logged_in(){
        return Boolean(this.#bearer)
    }

    async fetch(path, method, body){
        try{
            let request = new Request(
            this.#url + "/" + path + "?name=" + this.#name,
                {
                    method: method,
                    headers: {
                        "Content-Type": "application/json",
                        "Authorization": "Bearer " + this.#bearer,
                    },
                    body: JSON.stringify(body)
                }
            );

            let response = await fetch(request);

            if (!response.ok){
                throw new PrinterError(true, response.status, response.statusText)
            }

            let result = await response.json();

            if (result.error){
                if (result.error.code != "None"){
                    throw new PrinterError(false, result.error.code, result.error.message)
                }
            }

            return result.result
        } catch(e){
            // try to refresh token if unauthorised
            if (e.is_http_error && e.error_code == 401){
                // refresh token, will throw if not successful
                await this.refresh_token()
                // try to fetch again
                return await this.fetch(path, method, body)
            };

            // rethrow error
            throw e;
        }
    }

    store_tokens(){
        let tokens = JSON.parse(localStorage.getItem("tokens"));

        if (!tokens){
            tokens = {}
        }

        tokens[this.#name] = {"bearer": this.#bearer, "refresh_token": this.#refresh_token};

        localStorage.setItem("tokens", JSON.stringify(tokens));
    }

    async login(password){
        if (this.#bearer){
            return;
        }

        let request = new Request(
            this.#url + "/login?name=" + this.#name,
            {
                method: "POST",
                headers: {"Content-Type": "application/json"},
                body: JSON.stringify({
                    password: password
                })
            }
        );

        let response = await fetch(request);

        if (!response.ok){
            throw new PrinterError(true, response.status, response.statusText)
        }

        let result = await response.json();

        if (result.error){
            if (result.error.code != "None"){
                throw new PrinterError(false, result.error.code, result.error.message)
            }
        }

        this.#bearer = result.result.token;
        this.#refresh_token = result.result.refresh_token;

        this.store_tokens();
    }

    async refresh_token(){
        let request = new Request(
            this.#url + "/refresh_token?name=" + this.#name,
            {
                method: "POST",
                headers: {"Content-Type": "application/json"},
                body: JSON.stringify({
                    refresh_token: this.#refresh_token
                })
            }
        );

        let response = await fetch(request);

        if (!response.ok){
            throw new PrinterError(true, response.status, response.statusText)
        }

        let result = await response.json();

        if (result.error){
            if (result.error.code != "None"){
                throw new PrinterError(false, result.error.code, result.error.message)
            }
        }

        this.#bearer = result.result.token;
        this.#refresh_token = result.result.refresh_token;

        this.store_tokens();
    }

    async logout(){
        await this.fetch("logout", "POST", {})
    }

    async reset_password(new_password){
        await this.fetch("reset_password", "POST", {"new_password": new_password})
    }

    async get_info(){
        return await this.fetch("info", "GET", {})
    }

    async get_temperatures(){
        return await this.fetch("temperatures", "GET", {})
    }

    async emergency_stop(){
        return await this.fetch("emergency_stop", "POST", {})
    }
}