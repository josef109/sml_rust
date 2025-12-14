import datetime

import rrdtool


def gen_day_graph():
    fname = "db/ehz.rrd"
    watermark = datetime.datetime.now().strftime("%A %d %B %Y, %H\\:%M\\:%S")
    rrdtool.graph(
        "db/strom-tage.png",
        "--imgformat",
        "PNG",
        "--width",
        "1024",
        "--height",
        "612",
        "--title",
        "Strom letzte Stunde",
        "--vertical-label",
        "Energie Wh",
        "--units-exponent",
        "0",
        "--start",
        "-1h",
        "--right-axis",
        "10:-1000",
        "--right-axis-label",
        "Leistung W",
        "--lower-limit",
        "0",
        "--right-axis-format",
        "%4.0lf",
        "DEF:ein=%s:Einspeisung:AVERAGE" % fname,
        "DEF:za=%s:Bezug:AVERAGE" % fname,
        "DEF:zx=%s:Wirkleistung:AVERAGE" % fname,
        "CDEF:bb=zx,10000,+,100,/",
        "CDEF:ba=ein,36,*",
        "CDEF:bc=za,36,*",
        "LINE5:ba#FF0000:Einspeisung",
        "AREA:ba#7FFF7FFF:",
        "LINE5:bc#00FF00:Bezug",
        "AREA:bc#FF7F7F7F:",
        "LINE3:bb#FFF000:Leistung",
        # "AREA:bb#FFFF7F7F:",
        "COMMENT:%s" % watermark,
    )


gen_day_graph()
