import Plotly from 'plotly.js-dist-min'

class HeightMap{
    plot_div: HTMLDivElement;

    constructor(){
        this.plot_div = document.getElementById("heightmap-plot") as HTMLDivElement;

        let x_data: number[] = [];
        let y_data: number[] = [];
        let z_data: number[][] = [];

        for (let i=0; i< 10;i++){
            let z = [];
            for (let j=0;j<10;j++){
                y_data.push(j * 30);
                z.push(0)
            }
            x_data.push(i * 30);
            z_data.push(z);
        }

        var data: Plotly.Data[] = [{
            x: x_data,
            y: y_data,
            z: z_data,
            type: 'surface'
        }];

        var layout: Partial<Plotly.Layout> = {
            title: {
                text: 'heightmap'
            },
            autosize: false,
            width: this.plot_div.clientWidth,
            height: this.plot_div.clientHeight,
            margin: {
                l: 65,
                r: 50,
                b: 65,
                t: 90,
            },
            scene: {
                xaxis: {
                    range: [0, 330]
                },
                yaxis: {
                    range: [0, 330]
                },
                zaxis: {
                    range: [-0.5, 0.5]
                }
            }
        };

        Plotly.newPlot(this.plot_div, data, layout);
    }

    check_init(){
        var layout: Partial<Plotly.Layout> = {
            width: this.plot_div.clientWidth,
            height: this.plot_div.clientHeight
        };

        Plotly.relayout(this.plot_div, layout)
    }
}

export var heightMap = new HeightMap;