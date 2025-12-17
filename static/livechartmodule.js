/* ============================================================================
 *  LiveChartModule.js
 *  Chart.js 4.5.1
 * ========================================================================== */

/* ----------------------------- Dataset Keys ------------------------------ */

export const DATASETS = Object.freeze({
    LIVE: "live",
    MIN_POINT: "min_point",
    MAX: "max",
    AVG: "avg",
    MIN: "min",
    MAX_POINT: "maxpoint"
});

// /* --------------------------- Value Label Plugin --------------------------- */

// const valueLabelPlugin = {
//     id: "valueLabelPlugin",

//     afterDraw(chart) {
//         const { ctx, chartArea, scales } = chart;
//         const yScale = scales.y;

//         const datasets = chart.data.datasets;
//         let avgValue, maxValue, minValue;

//         for (let i = 0; i < datasets.length; i++) {
//             const d = datasets[i];
//             const v = d.data.at(-1);

//             if (v == null) continue;

//             switch (d.id) {
//                 case DATASETS.AVG:
//                     avgValue = v;
//                     break;
//                 case DATASETS.MAX:
//                     maxValue = v;
//                     break;
//                 case DATASETS.MIN:
//                     minValue = v;
//                     break;
//             }
//         }

//         if (avgValue == null) return;

//         ctx.save();
//         ctx.font = "600 11px Inter, system-ui, sans-serif";
//         ctx.textAlign = "right";
//         ctx.textBaseline = "middle";

//         const x = chartArea.right - 6;

//         const draw = (value, color, prefix = "") => {
//             ctx.fillStyle = color;
//             ctx.fillText(
//                 `${prefix}${Math.round(value)} W`,
//                 x,
//                 yScale.getPixelForValue(value) - 7
//             );
//         };

//         if (avgValue != null) draw(avgValue, "#8b93a3");
//         if (maxValue != null) draw(maxValue, "#8b93a3");
//         if (minValue != null) draw(minValue, "#8b93a3");

//         ctx.restore();
//     }
// };

/* ---------------------------- Module Factory ----------------------------- */

export function createLiveChart(ctx, theme) {

    /* --------- Dataset Lookup Cache (indexfrei & stabil) ---------- */

    const datasetCache = new Map();

    const cacheDatasets = (datasets) => {
        datasetCache.clear();
        for (let i = 0; i < datasets.length; i++) {
            datasetCache.set(datasets[i].id, datasets[i]);
        }
    };

    function average(ctx) {
        const values = ctx.chart.data.datasets[0].data;
        return values.reduce((a, b) => a + b, 0) / values.length;
    }

    function maxValue(ctx) {
        const dataset = ctx.chart.data.datasets[0];
        let max = (dataset.data[0] == undefined) ? 0 : dataset.data[0];

        dataset.data.forEach(function (el) {
            max = Math.max(max, el);
        });

        return max;
    }

    function minValue(ctx) {
        const dataset = ctx.chart.data.datasets[0];
        let min = (dataset.data[0] == undefined) ? 0 : dataset.data[0];

        dataset.data.forEach(function (el) {
            min = Math.min(min, el);
        });

        return min;
    }

    function maxIndex(ctx) {
        const max = maxValue(ctx);
        const dataset = ctx.chart.data.datasets[0];
        return dataset.data.indexOf(max);
    }

    function maxLabel(ctx) {
        return ctx.chart.data.labels[maxIndex(ctx)];
    }

    function minIndex(ctx) {
        const min = minValue(ctx);
        const dataset = ctx.chart.data.datasets[0];
        return dataset.data.indexOf(min);
    }

    function minLabel(ctx) {
        return ctx.chart.data.labels[minIndex(ctx)];
    }


    const annotation_avg_line = {
        type: 'line',
        borderColor: "#3ccf91",
        borderDash: [6, 6],
        borderDashOffset: 0,
        borderWidth: 1,
        label: {
            backgroundColor: 'transparent',
            color: (ctx) => ctx.chart.data.datasets[0].borderColor,
            display: true,
            content: (ctx) => '0 ' + average(ctx).toFixed(0) + ' W',
            position: 'end'
        },
        scaleID: 'y',
        value: (ctx) => average(ctx)
    };

    const annotation_max_point = {
        type: 'point',
        backgroundColor: 'transparent',
        borderColor: (ctx) => ctx.chart.data.datasets[0].borderColor,
        pointStyle: 'rectRounded',
        radius: 10,
        xValue: (ctx) => maxLabel(ctx),
        yValue: (ctx) => maxValue(ctx)
    };

    const annotation_min_point = {
        type: 'point',
        backgroundColor: 'transparent',
        borderColor: (ctx) => ctx.chart.data.datasets[0].borderColor,
        pointStyle: 'rectRounded',
        radius: 10,
        xValue: (ctx) => minLabel(ctx),
        yValue: (ctx) => minValue(ctx)
    };


    const annotation_max_label = {
        type: 'label',
        backgroundColor: 'transparent',
        color: (ctx) => ctx.chart.data.datasets[0].borderColor,
        callout: {
            display: true,
            borderColor: (ctx) => ctx.chart.data.datasets[0].borderColor,
        },
        content: (ctx) => maxValue(ctx).toFixed(1) + ' W',
        // font: {
        //     size: 16
        // },
        // padding: {
        //     top: 6,
        //     left: 6,
        //     right: 6,
        //     bottom: 12
        // },
        position: {
            x: (ctx) => maxIndex(ctx) <= 3 ? 'start' : maxIndex(ctx) >= ctx.chart.data.datasets[0].data.length - 10 ? 'end' : 'center',
            y: 'end'
        },
        // xValue: (ctx) => maxLabel(ctx),
        // yAdjust: -6,
        // yValue: (ctx) => maxValue(ctx)
        xAdjust: (ctx) => maxIndex(ctx) <= 3 ? 60 : maxIndex(ctx) >= 10 ? -60 : 0,
        xValue: (ctx) => maxLabel(ctx),
        yAdjust: -5,
        yValue: (ctx) => maxValue(ctx)
    };

    const annotation_min_label = {
        type: 'label',
        backgroundColor: 'transparent',
        color: (ctx) => ctx.chart.data.datasets[0].borderColor,
        callout: {
            display: true,
            borderColor: (ctx) => ctx.chart.data.datasets[0].borderColor
        },
        content: (ctx) => minValue(ctx).toFixed(1) + ' W',
        // font: {
        //     size: 16
        // },
        // padding: {
        //     top: 6,
        //     left: 6,
        //     right: 6,
        //     bottom: 12
        // },
        position: {
            x: (ctx) => minIndex(ctx) <= 3 ? 'start' : minIndex(ctx) >= ctx.chart.data.datasets[0].data.length - 10 ? 'end' : 'center',
            y: 'end'
        },
        // xValue: (ctx) => maxLabel(ctx),
        // yAdjust: -6,
        // yValue: (ctx) => maxValue(ctx)
        xAdjust: (ctx) => minIndex(ctx) <= 3 ? 60 : minIndex(ctx) >= 10 ? -60 : 0,
        xValue: (ctx) => minLabel(ctx),
        yAdjust: 40,
        yValue: (ctx) => minValue(ctx)
    };


    /* ---------------------------- Chart Init ----------------------------- */

    const chart = new Chart(ctx, {
        type: "line",

        data: {
            labels: [],

            datasets: [
                {
                    id: DATASETS.LIVE,
                    data: [],
                    borderWidth: 2,
                    tension: 0.1,
                    pointRadius: 0,
                    fill: true,
                },
                // {
                //     id: DATASETS.MIN_POINT,
                //     type: "scatter",
                //     data: [],
                //     pointRadius: 5,
                // },
                // {
                //     id: DATASETS.MAX,
                //     data: [],
                //     borderDash: [6, 6],
                //     borderWidth: 1,
                //     pointRadius: 0
                // },
                // {
                //     id: DATASETS.AVG,
                //     data: [],
                //     borderDash: [6, 6],
                //     borderWidth: 1,
                //     pointRadius: 0
                // },
                // {
                //     id: DATASETS.MIN,
                //     data: [],
                //     borderDash: [6, 6],
                //     borderWidth: 1,
                //     pointRadius: 0
                // },
                // {
                //     id: DATASETS.MAX_POINT,
                //     type: "scatter",
                //     data: [],
                //     pointRadius: 5
                // }
            ]
        },

        options: {
            plugins: {
                annotation: {
                    annotations: {
                        annotation_avg_line,
                        annotation_max_point,
                        annotation_max_label,
                        annotation_min_point,
                        annotation_min_label,
                    }
                },
                legend: { display: false },
                tooltip: {
                    intersect: false,
                    mode: "index"
                }

            },
            responsive: true,
            maintainAspectRatio: false,

            animation: false,

            scales: {
                x: {
                    grid: { color: theme.grid },
                    ticks: {
                        color: "#8b93a3",
                        autoSkip: true, // Überspringt Labels, wenn es zu voll wird
                        maxTicksLimit: 18 // Maximale Anzahl an sichtbaren Zeitstempeln
                    },
                    type: "time",
                    time: {
                        // 'unit' erzwingt, dass das Raster auf Sekunden basiert
                        unit: "second",
                        //stepSize: 30,
                        tooltipFormat: "HH:mm:ss",
                        displayFormats: {
                            "second": "HH:mm:ss"
                        }
                    }
                },
                y: {
                    grid: {
                        color: ({ tick }) => tick.value == 0 ? "#f2f9f9ff" : theme.grid,
                        //lineWidth: ({ tick }) => tick.value == 0 ? 5 : 1
                    },
                    ticks: {
                        color: "#8b93a3"
                    },
                    //beginAtZero: true,
                    suggestedMax: 1000,
                    suggestedMin: -50,
                }
            }
        }
    });

    /* ------------------------- Register Plugin -------------------------- */

    // if (!Chart.registry.plugins.get(valueLabelPlugin.id)) {
    //     Chart.register(valueLabelPlugin);
    // }

    cacheDatasets(chart.data.datasets);

    function applyChartTheme(t) {
        for (let i = 0; i < datasetCache.size; i++) {

            chart.data.datasets[i].borderColor = t.line;
            chart.data.datasets[i].backgroundColor = t.fill;
            chart.data.datasets[i].pointBackgroundColor = t.line;
        }
    }

    applyChartTheme(theme);


    /* --------------------------- Public API ------------------------------ */

    function pushData({
        label,
        power,
    }) {
        const data = chart.data;

        data.labels.push(label);

        datasetCache.get(DATASETS.LIVE).data.push(power);
        if (datasetCache.get(DATASETS.LIVE).data.length > 120) {
            data.labels.shift();
            datasetCache.get(DATASETS.LIVE).data.shift();
        }


        // const d = datasetCache.get(DATASETS.LIVE).data;
        // if (!d.length) return;
        // const min = Math.min(...d);
        // const min_i = d.indexOf(min);
        // datasetCache.get(DATASETS.MIN_POINT).data = [{ x: data.labels[min_i], y: min }];
        // datasetCache.get(DATASETS.MIN).data = data.labels.map(() => min);

        // const max = Math.max(...d);
        // const max_i = d.indexOf(max);
        // datasetCache.get(DATASETS.MAX_POINT).data = [{ x: data.labels[max_i], y: max }];
        // datasetCache.get(DATASETS.MAX).data = data.labels.map(() => max);

        // const sum = d.reduce((acc, value) => acc + value, 0);
        // const avg = sum / d.length;
        // datasetCache.get(DATASETS.AVG).data = data.labels.map(() => avg);

        const yTicks = chart.options.scales.y.ticks;

        // if (max > (yTicks.max ?? 0)) {
        //     yTicks.max = Math.ceil(max / 500) * 500;
        // }

        chart.update("none");
    }

    function destroy() {
        datasetCache.clear();
        chart.destroy();
    }

    return {
        chart,
        pushData,
        destroy
    };
}
