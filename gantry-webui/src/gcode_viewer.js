import GcodeViewer from '@sindarius/gcodeviewer';

class Viewer{
    constructor(){
        this.canvas = document.getElementById("GcodeViewerCanvas");
        this.gcodeViewer = null;
    }

    check_init(){
        if (this.gcodeViewer){
            return;
        }

        this.gcodeViewer = new GcodeViewer(this.canvas);
        this.gcodeViewer.init().then(()=>{
            window.addEventListener("resize", ()=>{this.gcodeViewer.resize()});
        });
    }

    resize(){
        this.gcodeViewer.resize();
    }

    before_render(){
        //gcodeViewer.gcodeProcessor.updateMinFeedColor(this.minFeedColor)
        //gcodeViewer.gcodeProcessor.updateMaxFeedColor(this.maxFeedColor)
        //gcodeViewer.gcodeProcessor.updateColorRate(this.minFeedRate, this.maxFeedRate)

        this.gcodeViewer.setZClipPlane(1000000, -1000000)
        this.gcodeViewer.setBackgroundColor("#000000")
        this.gcodeViewer.setProgressColor("#556b2f")
        this.gcodeViewer.bed.setBedColor("#ffbc0d")
        //gcodeViewer.gcodeProcessor.useSpecularColor(this.specular)
        //gcodeViewer.toggleTravels(this.travelMoves)
        //gcodeViewer.gcodeProcessor.setColorMode(this.renderMode)
        //gcodeViewer.gcodeProcessor.updateForceWireMode(this.lineMode)
        //gcodeViewer.gcodeProcessor.setVoxelMode(this.voxelMode)
        //gcodeViewer.updateRenderQuality(this.renderQuality)
        this.gcodeViewer.gcodeProcessor.useHighQualityExtrusion(true)
        //gcodeViewer.gcodeProcessor.setAlpha(0.5)
        this.gcodeViewer.gcodeProcessor.resetTools()
        //gcodeViewer.gcodeProcessor.g1AsExtrusion = this.g1AsExtrusion
        //gcodeViewer.gcodeProcessor.perimeterOnly = this.perimeterOnly
        //gcodeViewer.setZBelt(this.zBelt, this.zBeltAngle)
        //gcodeViewer.gcodeProcessor.progressMode = this.progressMode
        //gcodeViewer.gcodeProcessor.setTransparencyValue(0.5)
        this.gcodeViewer.setCursorVisiblity(true)
        this.gcodeViewer.bed.setRenderMode(0)
        this.gcodeViewer.axes.show(true)
        this.gcodeViewer.setWorkplaceVisiblity(true);
        this.gcodeViewer.displayViewBox(true);
        this.gcodeViewer.updateRenderQuality();
        //gcodeViewer.gcodeProcessor.persistTravels = this.persistTravel

        this.gcodeViewer.resize();
    }
}

export var gcodeViewer = new Viewer;