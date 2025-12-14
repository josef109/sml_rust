// --- Translation Dictionary ---
const translations = {
    de: {
        app_title: "Stromverbrauchsmonitor",
        nav_live: "Echtzeit",
        nav_hour: "Stunde",
        nav_day: "Tag",
        nav_week: "Woche",
        nav_status: "Status",
        card_live_title: "Live-Leistungsverlauf",
        card_hour_title: "Verlauf der letzten Stunde",
        card_day_title: "Verlauf der letzten 24 Stunden",
        card_week_title: "Verlauf der letzten Woche",
        card_status_title: "Aktuelle Messwerte & Status",
        last_update: "Letztes Update:",
        section_current_values: "Momentanwerte",
        stat_power: "Aktuelle Leistung",
        stat_meter: "Zählerstand (Bezug)",
        stat_direction: "Netzrichtung",
        status_init: "Initialisiere...",
        footer_note: "Die Grafiken werden alle 30 Sekunden aktualisiert.",
        // Dynamic Logic Strings
        chart_label_power: "Wirkleistung (W)",
        chart_label_consump: "Verbrauch/Intervall (Wh)",
        chart_axis_time: "Zeit",
        chart_axis_power: "Watt (W)",
        chart_axis_consump: "Verbrauch/Intervall (Wh)",
        chart_title: "Echtzeit-Leistungsverlauf",
        status_feed_in: "Einspeisung",
        status_consumption: "Bezug"
    },
    en: {
        app_title: "Power Consumption Monitor",
        nav_live: "Real-time",
        nav_hour: "Hour",
        nav_day: "Day",
        nav_week: "Week",
        nav_status: "Status",
        card_live_title: "Live Power History",
        card_hour_title: "Last Hour History",
        card_day_title: "Last 24 Hours History",
        card_week_title: "Last Week History",
        card_status_title: "Current Values & Status",
        last_update: "Last Update:",
        section_current_values: "Current Values",
        stat_power: "Current Power",
        stat_meter: "Meter Reading (Grid)",
        stat_direction: "Grid Direction",
        status_init: "Initializing...",
        footer_note: "Charts are updated every 30 seconds.",
        // Dynamic Logic Strings
        chart_label_power: "Active Power (W)",
        chart_label_consump: "Consump./Interval (Wh)",
        chart_axis_time: "Time",
        chart_axis_power: "Watt (W)",
        chart_axis_consump: "Consump./Interval (Wh)",
        chart_title: "Real-time Power History",
        status_feed_in: "Grid Feed-in",
        status_consumption: "Consumption"
    }
};

// Global settings
let currentLang = localStorage.getItem('appLang') || 'de'; // Default to DE
let liveChart;

// --- Language Switching Logic ---
function setLanguage(lang) {
    if (!translations[lang]) return;
    currentLang = lang;
    localStorage.setItem('appLang', lang); // Persist selection

    // 1. Update static HTML elements with data-i18n attribute
    $('[data-i18n]').each(function () {
        const key = $(this).attr('data-i18n');
        if (translations[lang][key]) {
            $(this).text(translations[lang][key]);
        }
    });

    // 2. Update Moment.js Locale
    moment.locale(lang);

    // 3. Update Chart.js Labels if chart exists
    if (liveChart) {
        liveChart.options.title.text = translations[lang].chart_title;
        liveChart.options.scales.xAxes[0].scaleLabel.labelString = translations[lang].chart_axis_time;
        liveChart.options.scales.yAxes[0].scaleLabel.labelString = translations[lang].chart_axis_power;
        liveChart.options.scales.yAxes[1].scaleLabel.labelString = translations[lang].chart_axis_consump;
        liveChart.data.datasets[0].label = translations[lang].chart_label_power;
        liveChart.data.datasets[1].label = translations[lang].chart_label_consump;
        liveChart.update();
    }

    // 4. Update Button Styles
    $('.lang-btn').removeClass('active');
    $('#btn-' + lang).addClass('active');

    // 5. Update Static Images (Assuming file naming convention: image-de.png / image-en.png)
    updateImage();
}

// --- SSE-Integration ---
const eventSource = new EventSource("/events");

eventSource.onmessage = function (event) {
    try {
        const data = JSON.parse(event.data);

        // --- Status Page Updates ---
        $('#last-update').text(data.time);

        // Format Power
        let powerW = data.value.toFixed(1);
        if (currentLang === 'de') powerW = powerW.replace('.', ',');
        $('#leistung').text(powerW);

        // Meter Reading
        if (data.total_energy) {
            let energy = data.total_energy.toFixed(1);
            if (currentLang === 'de') energy = energy.replace('.', ',');
            $('#bezug').text(energy);
        }

        // Feed-in Status Logic
        const isFeedIn = data.is_feed_in !== undefined ? data.is_feed_in : (data.value < 0);

        // Use translation dictionary for status text
        const statusText = isFeedIn ? translations[currentLang].status_feed_in : translations[currentLang].status_consumption;

        $('#einspeisung').text(statusText);

        if (isFeedIn) {
            $('#einspeisung-status')
                .removeClass('alert-success')
                .addClass('alert-warning');
            $('#einspeisung-status .status-indicator')
                .removeClass('status-true')
                .addClass('status-false');
        } else {
            $('#einspeisung-status')
                .removeClass('alert-danger')
                .addClass('alert-success');
            $('#einspeisung-status .status-indicator')
                .removeClass('status-false')
                .addClass('status-true');
        }

        // --- Live Chart Update ---
        if (liveChart) {
            const maxDataPoints = 50;
            if (liveChart.config.data.labels.length >= maxDataPoints) {
                liveChart.config.data.labels.shift();
                liveChart.config.data.datasets[0].data.shift();
                liveChart.config.data.datasets[1].data.shift();
            }

            liveChart.config.data.labels.push(data.time);
            liveChart.config.data.datasets[0].data.push(data.value);
            liveChart.config.data.datasets[1].data.push(data.value2);
            liveChart.update();
        }

    } catch (e) {
        console.error("Error processing SSE data:", e, event.data);
    }
};

// Function to update static images
function updateImage() {
    $('img').each(function () {
        const timestamp = new Date().getTime();
        let src = $(this).attr('src').split('?')[0];

        // Logic to swap language in filename (e.g., strom-tag-de.png <-> strom-tag-en.png)
        // This assumes you have matching images for English on the server.
        if (currentLang === 'en' && src.includes('-de')) {
            src = src.replace('-de', '-en');
        } else if (currentLang === 'de' && src.includes('-en')) {
            src = src.replace('-en', '-de');
        }

        $(this).attr('src', src + '?' + timestamp);
    });
    // Remove previous timeout if exists to avoid stacking
    if (window.imageUpdateTimeout) clearTimeout(window.imageUpdateTimeout);
    window.imageUpdateTimeout = setTimeout(updateImage, 60000);
}

// Function to create the Live Chart
function createLiveChart() {
    const config = {
        type: 'line',
        data: {
            labels: [],
            datasets: [{
                label: translations[currentLang].chart_label_power, // Use variable
                yAxisID: "Wirkleistung-y-axis",
                backgroundColor: 'rgba(75, 108, 183, 0.5)',
                borderColor: 'rgba(75, 108, 183, 1)',
                data: [],
                fill: false,
                borderWidth: 2,
                pointRadius: 0
            },
            {
                label: translations[currentLang].chart_label_consump, // Use variable
                yAxisID: "Zähler-y-axis",
                backgroundColor: 'rgba(40, 167, 69, 0.2)',
                borderColor: 'rgba(40, 167, 69, 1)',
                data: [],
                fill: true,
                borderWidth: 2,
                pointRadius: 0
            }]
        },
        options: {
            responsive: true,
            title: {
                display: true,
                text: translations[currentLang].chart_title // Use variable
            },
            tooltips: {
                mode: 'index',
                intersect: false,
            },
            hover: {
                mode: 'nearest',
                intersect: true
            },
            scales: {
                xAxes: [{
                    display: true,
                    scaleLabel: {
                        display: true,
                        labelString: translations[currentLang].chart_axis_time
                    },
                    gridLines: {
                        display: false
                    },
                    ticks: {
                        callback: function (value, index, values) {
                            if (index % 2 !== 0) return null;
                            if (typeof value === 'string') return value.split(':');
                            return value;
                        },
                        skipFalses: true,
                        minRotation: 0,
                        maxRotation: 0,
                    }
                }],
                yAxes: [{
                    "id": "Wirkleistung-y-axis",
                    display: true,
                    position: 'left',
                    scaleLabel: {
                        display: true,
                        labelString: translations[currentLang].chart_axis_power
                    },
                    ticks: {
                        fontColor: "rgba(75, 108, 183, 1)",
                        suggestedMax: 1000,
                        beginAtZero: true,
                    },
                    gridLines: {
                        color: 'rgba(75, 108, 183, 0.1)'
                    }
                },
                {
                    "id": "Zähler-y-axis",
                    display: true,
                    position: 'right',
                    scaleLabel: {
                        display: true,
                        labelString: translations[currentLang].chart_axis_consump
                    },
                    ticks: {
                        fontColor: "rgba(40, 167, 69, 1)",
                        suggestedMax: 2,
                        beginAtZero: true,
                    },
                    gridLines: {
                        display: false
                    }
                }]
            },
            animation: {
                duration: 0
            }
        }
    };

    const context = document.getElementById('liveChart').getContext('2d');
    return new Chart(context, config);
}


$(document).ready(function () {
    // 1. Initialize language first
    setLanguage(currentLang);

    // 2. Initialize the chart
    liveChart = createLiveChart();
});