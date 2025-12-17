import { createLiveChart } from "./livechartmodule.js";



const CHART_THEMES = {
    neutral: { line: "#9ca3af", fill: "rgba(156,163,175,.15)", grid: "rgba(148,163,184,.15)" },
    export: { line: "#f0c36d", fill: "rgba(240,195,109,.2)", grid: "rgba(240,195,109,.2)" },
    import: { line: "#3ccf91", fill: "rgba(60,207,145,.2)", grid: "rgba(60,207,145,.2)" },
    error: { line: "#ef6b6b", fill: "rgba(239,107,107,.2)", grid: "rgba(239,107,107,.25)" }
};

let currentYMax = null;

const ctx = document.getElementById("liveChart").getContext("2d");

function updateGridStatus(type, text) {
    const box = document.getElementById("einspeisung-status");
    const lbl = document.getElementById("einspeisung_text");
    box.className = "alert-status status-" + type;
    lbl.textContent = text;
    //applyChartTheme(type);
}

const { pushData } = createLiveChart(ctx, CHART_THEMES.import);

// --- NEU: Initialisierung der Live-Daten ---
async function initLiveChart() {
    try {
        // Wir laden die letzten 360 Sekunden (6 Minuten), 
        // was bei einem 3-Sekunden-Raster ca. 120 Datenpunkten entspricht.
        const response = await fetch('/api/live?ds=Wirkleistung&seconds=360');
        if (!response.ok) throw new Error("Fehler beim Laden der Initialdaten");

        const history = await response.json();

        // history sollte ein Array von Objekten sein, z.B. [{time: "12:00:01", value: 123.4}, ...]
        // Je nachdem wie dein Rust-Handler live_data die Daten strukturiert:
        history.forEach(point => {
            pushData({
                label: point[0], // Der Zeitstempel aus der RRD
                power: point[1] / 10  // Der Wert (evtl. noch durch 10 teilen, falls nötig)
            });
            //console.log("Datum ", point[0]);
        });

        console.log("Historische Daten geladen:", history.length);
    } catch (e) {
        console.error("Initialisierungsfehler:", e);
    }
}

// Starte das Laden der alten Daten
initLiveChart();
// pushData({
//     label: Date.now() - 20000, // Der Zeitstempel aus der RRD
//     power: 100  // Der Wert (evtl. noch durch 10 teilen, falls nötig)
// });
// pushData({
//     label: Date.now() - 10000, // Der Zeitstempel aus der RRD
//     power: 200  // Der Wert (evtl. noch durch 10 teilen, falls nötig)
// });
// pushData({
//     label: Date.now(), // Der Zeitstempel aus der RRD
//     power: 300  // Der Wert (evtl. noch durch 10 teilen, falls nötig)
// });

// --- SSE-Integration ---
const eventSource = new EventSource("/events");

eventSource.onmessage = function (event) {
    try {
        const data = JSON.parse(event.data);
        const zeit = new Date(data.time);

        pushData({
            label: zeit,
            power: data.power / 10,
        });
        document.getElementById("power").textContent = data.power / 10;

        $('#last-update').text(zeit.toLocaleTimeString());
        $('#export').text(data.export / 10)
        $('#import').text(data.import / 10)
        $('#export-diff').text(data.export_diff / 10)
        $('#import-diff').text(data.import_diff / 10)

        updateGridStatus(data.power > 0 ? "import" : "export", data.power > 0 ? "Bezug" : "Einspeisung");
    } catch (e) {
        console.error("Error processing SSE data:", e, event.data);
    }
};

/* Demo */
// setInterval(() => {
//     const x = Math.floor((Math.random() - 0.1) * 7000) / 10.0;
//     const v = Math.floor((Math.sin(new Date().getSeconds() / 60 * Math.PI) - 0.1) * 70000) / 10.0;

//     //addLiveValue(new Date().toLocaleTimeString(), v);
//     document.getElementById("power").textContent = v;

//     $('#last-update').text(new Date().toLocaleTimeString());
//     $('#export').text(10000.0)
//     $('#import').text(100.0)
//     updateGridStatus(v > 0 ? "import" : "export", v > 0 ? "Bezug" : "Einspeisung");
//     pushData({
//         label: new Date().toLocaleTimeString(),
//         power: x,
//     });
// }, 2000);
