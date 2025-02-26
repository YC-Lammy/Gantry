




export class Server{
    constructor(url){

    }

    async getPrinterInfo(){
        return [
            {
                name: "Trident",
                login_required: false,
            }
        ]
    }
}