import {Printer as PrinterIface} from './printer';
import { Server as ServerIface } from './server';

export * from "./dashboard";
export * from "./heightmap";
import { heightMap } from './heightmap';
export * from "./gcode_viewer";
import { gcodeViewer } from './gcode_viewer';

export var Printer;
export var Server = new ServerIface(window.location.host);

export function open_tab(event, id){
    let links = document.getElementsByClassName("tablinks");

    for (let i=0;i<links.length;i++){
        let link = links[i];
        link.classList.remove("active");
    }

    event.target.classList.add("active");

    let contents = document.getElementsByClassName("tabcontent");

    for (let i=0;i<contents.length;i++){
        let content = contents[i];
        content.classList.remove("active");
    }

    document.getElementById(id).classList.add("active");

    switch (id){
        case "Heightmap":
            heightMap.check_init();
            break;
        case "Viewer":
            gcodeViewer.check_init();
            break;
        default:
    }
}

export async function resolve_printer_login(){
    let tokens = JSON.parse(localStorage.getItem("tokens"));

    if (!tokens){
        tokens = {};
    }

    let printers = await Server.getPrinterInfo();

    let grid = document.getElementById("PrintersGrid");

    let template = document.getElementById("PrintersOptionTemplate"); //as HTMLTemplateElement;

    for (let info of printers){
        // clone from template
        let option = template.content.cloneNode(true); //as HTMLElement;
        // set text
        option.querySelector(".PrintersOptionText").textContent = info.name;
        // set callback
        option.querySelector(".PrintersOptionButton")
        .addEventListener("click", async (event)=>{
            let token = tokens[info.name];
            let printer = new PrinterIface(info.name, window.location.host);

            if (info.login_required){
                if (token){
                    if (!printer.check_tokens(token.bearer, token.refresh_token)){
                        return;
                    }
                } else{
                    try{
                        let password = prompt("enter password for " + info.name);
                        await printer.login(password);
                    } catch(e){
                        alert(e.toSting() + "\nincorrect password, please try again");
                        return;
                    }
                }
            }

            Printer = printer;

            let options = document.getElementsByClassName("PrintersOption");

            for (let i=0;i<options.length;i++){
                options[i].classList.remove("active");
            }

            event.target.classList.add("active");
        })
        grid.appendChild(option);
    }
}
resolve_printer_login();