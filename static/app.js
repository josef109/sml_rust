import { createLiveChart } from "./livechartmodule.js";

const CHART_THEMES = {
    neutral: { line: "#9ca3af", fill: "rgba(156,163,175,.1)", grid: "rgba(148,163,184,.1)" },
    export: { line: "#f0c36d", fill: "rgba(240,195,109,.15)", grid: "rgba(240,195,109,.1)" },
    import: { line: "#3ccf91", fill: "rgba(60,207,145,.15)", grid: "rgba(60,207,145,.1)" },
    error: { line: "#ef6b6b", fill: "rgba(239,107,107,.2)", grid: "rgba(239,107,107,.2)" }
};

const ctx = document.getElementById("liveChart").getContext("2d");
const { chart, pushData, applyChartTheme } = createLiveChart(ctx, CHART_THEMES.neutral);

// 1. Initialisierung: Letzte Daten laden
async function initLiveChart() {
    try {
        const response = await fetch('/api/live?ds=Wirkleistung&seconds=600');
        const history = await response.json();
        history.forEach(p => pushData({ label: p[0], power: p[1] / 10 }));
    } catch (e) { console.error("Initialisierung fehlgeschlagen", e); }
}

initLiveChart();

let watchdogTimer;

function startWatchdog() {
    // Falls bereits ein Timer läuft, stoppen
    clearTimeout(watchdogTimer);

    // Nach 5 Sekunden ohne Daten das Error-Theme setzen
    watchdogTimer = setTimeout(() => {
        console.warn("Seit 5s keine Daten empfangen!");
        applyChartTheme(CHART_THEMES.error);
        chart.update("none");
        document.getElementById("power").textContent = "OFFLINE";
    }, 5000);
}

// 2. Echtzeit-Updates via SSE
const eventSource = new EventSource("/events");
eventSource.onmessage = (event) => {
    const data = JSON.parse(event.data);
    const p = data.power / 10;
    const zeit = new Date(data.time);
    startWatchdog();

    // Live-Chart Thema anpassen
    if (p > 20) applyChartTheme(CHART_THEMES.import);
    else if (p < -20) applyChartTheme(CHART_THEMES.export);
    else applyChartTheme(CHART_THEMES.neutral);

    pushData({ label: zeit, power: p });

    // UI-Elemente aktualisieren
    document.getElementById("power").textContent = p.toFixed(1);
    document.getElementById("last-update").textContent = zeit.toLocaleTimeString();
    document.getElementById("import").textContent = (data.import / 10).toFixed(1);
    document.getElementById("export").textContent = (data.export / 10).toFixed(1);
    document.getElementById("import-diff").textContent = (data.import_diff / 10).toFixed(1);
    document.getElementById("export-diff").textContent = (data.export_diff / 10).toFixed(1);

    const box = document.getElementById("einspeisung-status");
    const lbl = document.getElementById("einspeisung_text");
    if (p > 0) {
        box.className = "alert-status status-import";
        lbl.textContent = "Netzbezug";
    } else {
        box.className = "alert-status status-export";
        lbl.textContent = "Einspeisung";
    }
};

// 3. Alle Verlaufs-Bilder regelmäßig neu laden
function refreshAllGraphs() {
    const ts = Date.now();
    $('.rrd-graph').each(function () {
        const baseSrc = $(this).attr('src').split('?')[0];
        $(this).attr('src', baseSrc + '?t=' + ts);
    });
    console.log("Dashboard Grafiken aktualisiert: " + new Date().toLocaleTimeString());
}

// Alle 60 Sekunden auffrischen
setInterval(refreshAllGraphs, 60000);

startWatchdog();

if (typeof mediumZoom === 'function') {

    mediumZoom('.rrd-graph', {
        margin: 24,          // Abstand zum Bildschirmrand beim Zoomen
        background: '#0a0d11', // Dunkler Hintergrund passend zum Theme
        scrollOffset: 0,     // Verhindert Scrollen beim Zoomen
    });

    console.log("Zoom-Funktion für Graphen aktiviert.");
} else {
    console.warn("medium-zoom Bibliothek wurde nicht gefunden.");
}

async function updateHistory() {
    try {
        const response = await fetch('/api/history');
        const data = await response.json();
        const container = document.getElementById('history-body');
        container.innerHTML = '';

        // Wir iterieren durch die Daten
        data.forEach((row, index) => {
            const tr = document.createElement('tr');

            // Aktuelle Zählerstände (in 0.1 Wh Einheiten laut deinem Rust-Code)
            const currentImport = parseInt(row[1]);
            const currentExport = parseInt(row[2]);

            // Holen den Stand vom "Vortag" (der in der Liste danach kommt, da neueste zuerst)
            const nextRow = data[index + 1];

            let diffImport = "---";
            let diffExport = "---";

            if (nextRow) {
                const prevImport = parseInt(nextRow[1]);
                const prevExport = parseInt(nextRow[2]);

                // Berechnung der Differenz (Verbrauch/Einspeisung des Tages)
                // Umrechnung von 0.1 Wh in Wh (/ 10) oder kWh (/ 10000)
                diffImport = ((currentImport - prevImport) / 10).toFixed(1);
                diffExport = ((currentExport - prevExport) / 10).toFixed(1);
            }
            else
                return;

            // Absolute Zählerstände für die Anzeige (in Wh)
            const importTotal = (currentImport / 10).toFixed(1);
            const exportTotal = (currentExport / 10).toFixed(1);

            // Datum formatieren
            let datum = new Date(row[0]);
            datum.setDate(datum.getDate() - 1);

            //const d = setDate(new Date(row[0]).getDate());
            const date = datum.toLocaleDateString("de-DE", {
                day: '2-digit',
                month: '2-digit',
                year: 'numeric'
            });

            tr.innerHTML = `
                <td>${date}</td>
                <td class="text-right text-muted small">${importTotal}</td>
                <td class="text-right font-weight-bold text-success">${diffImport}</td>
                <td class="text-right text-muted small">${exportTotal}</td>
                <td class="text-right font-weight-bold text-warning">${diffExport}</td>
            `;
            container.appendChild(tr);
        });
    } catch (e) {
        console.error("History error:", e);
    }
}

updateHistory();