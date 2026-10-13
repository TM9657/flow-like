export const richMediaMarkdown = `## Drei Richtungen für deine Auswahl

Diese fiktiven Fahrzeuge und Bewertungen zeigen die Darstellung von Bildern, Empfehlungen und Vergleichen. Die Illustrationen stellen keine realen Fahrzeugmodelle dar.

\`\`\`gallery
{
  "title": "Bilder zur Orientierung",
  "items": [
    {
      "src": "/assets/sedan-a.svg",
      "alt": "Illustration einer dunkelblauen Limousine in Seitenansicht",
      "caption": "Modell A · ruhige, klassische Form",
      "credit": "Illustration für die Vorschau"
    },
    {
      "src": "/assets/sedan-b.svg",
      "alt": "Illustration einer anthrazitfarbenen Limousine in Seitenansicht",
      "caption": "Modell B · längere Silhouette",
      "credit": "Illustration für die Vorschau"
    },
    {
      "src": "/assets/sedan-c.svg",
      "alt": "Illustration einer hellen Limousine in Seitenansicht",
      "caption": "Modell C · flacher Dachverlauf",
      "credit": "Illustration für die Vorschau"
    }
  ]
}
\`\`\`

### Meine Reihenfolge im Beispiel

Die Rangfolge folgt hier der angenommenen Priorität: zuerst Komfort, dann Alltag und Fahrgefühl.

\`\`\`recommendations
{
  "items": [
    {
      "title": "Modell A",
      "subtitle": "Komfortorientierte Limousine",
      "badge": "Passt zur Priorität",
      "description": "Im Beispiel wäre das meine erste Auswahl für lange Strecken. Das Fahrgefühl bleibt ruhig, während die Übersicht im Innenraum den Alltag erleichtert. Eine Probefahrt müsste bestätigen, ob die Sitzposition zu dir passt.",
      "image": {
        "src": "/assets/sedan-a.svg",
        "alt": "Dunkelblaue Limousine, illustrative Seitenansicht"
      },
      "specs": [
        {
          "label": "Leistung",
          "value": "400 PS"
        },
        {
          "label": "Antrieb",
          "value": "Allrad"
        },
        {
          "label": "Budget",
          "value": "bis 55.000 €"
        }
      ],
      "href": "https://example.com/modell-a"
    },
    {
      "title": "Modell C",
      "subtitle": "Sportlichere Alternative",
      "badge": "Mehr Fahrgefühl",
      "description": "Diese Alternative gewichtet im Beispiel die direkte Rückmeldung stärker. Sie passt, wenn du für das Fahrgefühl beim Federungskomfort etwas Spielraum lässt. Die Ausstattungsdetails bleiben entscheidend.",
      "image": {
        "src": "/assets/sedan-c.svg",
        "alt": "Helle Limousine, illustrative Seitenansicht"
      },
      "specs": [
        {
          "label": "Leistung",
          "value": "440 PS"
        },
        {
          "label": "Antrieb",
          "value": "Allrad"
        },
        {
          "label": "Budget",
          "value": "bis 55.000 €"
        }
      ],
      "href": "https://example.com/modell-c"
    }
  ]
}
\`\`\`

### Komfort im direkten Vergleich

\`\`\`comparison
{
  "title": "Illustrative Einschätzung",
  "description": "Beispielbewertung auf einer Skala von 0 bis 5. Die Werte sind keine Messdaten.",
  "max": 5,
  "items": [
    {
      "label": "Modell A",
      "value": 4.5,
      "valueLabel": "Sehr gut",
      "detail": "Im Beispiel mit Schwerpunkt auf Langstreckenkomfort."
    },
    {
      "label": "Modell B",
      "value": 4,
      "valueLabel": "Gut",
      "detail": "Ausgewogene Annahme für Stadt und Autobahn."
    },
    {
      "label": "Modell C",
      "value": 3.5,
      "valueLabel": "Gut",
      "detail": "Im Beispiel zugunsten direkterer Rückmeldung abgestimmt."
    }
  ]
}
\`\`\`

### Einzelbild mit Bildunterschrift

<img src="/assets/sedan-b.svg" alt="Anthrazitfarbene Limousine als eigenständige Illustration" caption="Modell B in Seitenansicht. Bildunterschriften bleiben direkt am zugehörigen Bild." credit="Illustration für die Vorschau" />

Ein einzelnes Bild und eine Galerie sollen sich in dieselbe Antwort einfügen. Die nächste Textpassage folgt dem normalen Absatzabstand.
`;

export const chartsMarkdown = `## Messwerte als Diagramm

Diese Diagramme verwenden **illustrative Beispieldaten** für fiktive Modelle. Sie prüfen Achsen, Legenden, Farben und die Darstellung in schmalen Antwortspalten.

### Leistung im Vergleich

\`\`\`nivo
type: bar
title: Leistung · Beispielwerte
layout: horizontal
height: 320
showLegend: false
animate: false
---
Modell,PS
Modell C,440
Modell B,340
Modell A,400
\`\`\`

Die horizontale Darstellung hält die Modellnamen neben den Balken. Ein Tooltip zeigt den zugrunde liegenden Wert.

### Verbrauch über drei Fahrprofile

\`\`\`plotly
{
  "data": [
    {"type": "scatter", "name": "Modell A", "mode": "lines+markers", "x": ["Stadt", "Landstraße", "Autobahn"], "y": [9.2, 6.4, 7.8]},
    {"type": "scatter", "name": "Modell C", "mode": "lines+markers", "x": ["Stadt", "Landstraße", "Autobahn"], "y": [10.4, 7.2, 8.6]}
  ],
  "layout": {
    "title": {"text": "Verbrauch · illustrative Werte"},
    "yaxis": {"title": {"text": "Liter / 100 km"}},
    "margin": {"l": 58, "r": 18, "t": 54, "b": 74},
    "legend": {"orientation": "h", "x": 0, "y": -0.2},
    "hovermode": "x unified"
  },
  "config": {"displayModeBar": false, "responsive": true}
}
\`\`\`

**Lesart:** Die Linien verbinden Beispielwerte für unterschiedliche Fahrprofile. Sie bilden keine Zeitreihe und keinen gemessenen Verbrauch ab.
`;
