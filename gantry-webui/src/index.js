import GcodeViewer from '@sindarius/gcodeviewer';

import {Printer} from './printer';
import { Server } from './server';

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

    if (id == "Viewer"){
        document.gcodeViewer.resize();
    }
}



document.Server = new Server(window.location.host);

export async function resolve_printer_login(close_button){
    if (close_button){
        document.getElementById("PrinterSelectDialogCloseButton").style.display = "block";
    } else{
        document.getElementById("PrinterSelectDialogCloseButton").style.display = "none";
    }

    document.getElementById("PrinterSelectDialog").classList.add("active");

    let tokens = JSON.parse(localStorage.getItem("tokens"));

    if (!tokens){
        tokens = {};
    }

    let printers = await document.Server.getPrinterInfo();

    let grid = document.getElementById("PrinterSelectDialogGrid");

    let template = document.getElementById("PrinterSelectDialogOptionTemplate");

    for (let info of printers){
        // clone from template
        let option = template.content.cloneNode(true);
        // set text
        option.querySelector(".PrinterSelectDialogText").textContent = info.name;
        // set callback
        option.querySelector(".PrinterSelectDialogButton").onclick = async ()=>{
            let token = tokens[info.name];
            document.Printer = new Printer(info.name, window.location.host);

            if (token){
                document.Printer.check_tokens(token.bearer, token.refresh_token);
            }
            
            if (info.login_required){
                try{
                    let password = prompt("enter your password");
                    await document.Printer.login(password);
                } catch(e){
                    alert(e.toSting() + "\nwrong password, please try again")
                }
            }
            
            document.getElementById("PrinterSelectDialog").classList.remove("active");
        }
        grid.childNodes.appendChild(option);
    }
}

resolve_printer_login();

async function init_gcode_viewer(canvas){
    let gcodeViewer = new GcodeViewer(canvas);
    await gcodeViewer.init();

    window.addEventListener("resize", ()=>{gcodeViewer.resize()});
    canvas.addEventListener("visibilitychange", ()=>{gcodeViewer.resize()})

    //gcodeViewer.gcodeProcessor.updateMinFeedColor(this.minFeedColor)
    //gcodeViewer.gcodeProcessor.updateMaxFeedColor(this.maxFeedColor)
    //gcodeViewer.gcodeProcessor.updateColorRate(this.minFeedRate, this.maxFeedRate)

    gcodeViewer.setZClipPlane(1000000, -1000000)
    gcodeViewer.setBackgroundColor("#000000")
    gcodeViewer.setProgressColor("#556b2f")
    gcodeViewer.bed.setBedColor("#ffbc0d")
    //gcodeViewer.gcodeProcessor.useSpecularColor(this.specular)
    //gcodeViewer.toggleTravels(this.travelMoves)
    //gcodeViewer.gcodeProcessor.setColorMode(this.renderMode)
    //gcodeViewer.gcodeProcessor.updateForceWireMode(this.lineMode)
    //gcodeViewer.gcodeProcessor.setVoxelMode(this.voxelMode)
    //gcodeViewer.updateRenderQuality(this.renderQuality)
    gcodeViewer.gcodeProcessor.useHighQualityExtrusion(true)
    //gcodeViewer.gcodeProcessor.setAlpha(0.5)
    gcodeViewer.gcodeProcessor.resetTools()
    //gcodeViewer.gcodeProcessor.g1AsExtrusion = this.g1AsExtrusion
    //gcodeViewer.gcodeProcessor.perimeterOnly = this.perimeterOnly
    //gcodeViewer.setZBelt(this.zBelt, this.zBeltAngle)
    //gcodeViewer.gcodeProcessor.progressMode = this.progressMode
    //gcodeViewer.gcodeProcessor.setTransparencyValue(0.5)
    gcodeViewer.setCursorVisiblity(true)
    gcodeViewer.bed.setRenderMode(0)
    gcodeViewer.axes.show(true)
    gcodeViewer.setWorkplaceVisiblity(true);
    gcodeViewer.displayViewBox(true);
    gcodeViewer.updateRenderQuality();
    //gcodeViewer.gcodeProcessor.persistTravels = this.persistTravel

    gcodeViewer.resize();

    setTimeout(()=>gcodeViewer.resize(), 100)

    document.gcodeViewer = gcodeViewer;
}

let gcode_viewer_canvas = document.getElementById("GcodeViewerCanvas");

init_gcode_viewer(gcode_viewer_canvas);

prompt("hello");