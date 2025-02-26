import Plotly from 'plotly.js-dist-min'

class DashboardTemperatures{
    plot_div: HTMLDivElement;
    /// names of the temperature sensors
    names: string[] = [];
    /// time of each data point
    x_time: Date[] = [];
    /// array of array of data point
    y_datas:Float32Array[] = [];
    /// colour for each trace
    colours: string[] = [];

    constructor(){
        this.plot_div = document.getElementById("dashboard-temperatures-chart") as HTMLDivElement;

        const MS_PER_MINUTE = 60000;
        let end_time = new Date;
        // show 20 minutes
        let start_time = new Date(end_time.getTime() - 20 * MS_PER_MINUTE);

        let layout: Partial<Plotly.Layout> = {
            title: {
                text: "temperature(°C)"
            },
            xaxis: {
                range: [start_time, end_time],
                type: "date"
            },
            yaxis: {
                range: [0, 500]
            },
        };

        let config = {
            displaylogo: false
        }

        Plotly.newPlot(
            this.plot_div,
            [],
            layout,
            config
        );

        // start updating plot
        setInterval(()=>{this.update_plot()}, 100);
    }

    check_init(){

    }

    update_plot(){
        const MS_PER_MINUTE = 60000;
        let end_time = new Date;
        // show 20 minutes
        let start_time = new Date(end_time.getTime() - 20 * MS_PER_MINUTE);

        let traces: Plotly.Data[] = [];

        for (let i=0;i<this.y_datas.length;i++){
            traces.push({
                type: "scatter",
                mode: "lines",
                name: this.names[i],
                x: this.x_time,
                y: this.y_datas[i],
                line: {
                    color: this.colours[i]
                }
            });
        }

        let layout: Partial<Plotly.Layout> = {
            title: {
                text: "temperature(°C)"
            },
            xaxis: {
                range: [start_time, end_time],
                type: "date"
            },
            yaxis: {
                range: [0, 500],
                autorange: this.y_datas.length >= 1
            }
        };

        let config = {
            displaylogo: false
        }

        Plotly.react(
            this.plot_div,
            traces,
            layout,
            config
        );
    }
}
export var dashboardTemperatures = new DashboardTemperatures;