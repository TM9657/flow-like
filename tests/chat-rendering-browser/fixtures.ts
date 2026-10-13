export const comparisonMarkdown = `## Was heißt das für deinen Vergleich?

Wenn **Diesel, Allrad und mehr als 350 PS** gleichzeitig Pflicht sind, lohnt sich ein Blick auf Motorisierung und Ausstattung. Die Modellbezeichnung allein beantwortet die Frage nach dem Head-up-Display noch nicht.

### Motoren und Fahrleistungen

| Modell | Motor | PS | 0–100 km/h ca. |
| :--- | :--- | ---: | ---: |
| Panamera 4 | 3.0 V6 Benzin | 330 | 5,3 s |
| Panamera 4S | 2.9 V6 Biturbo | 440 | 4,2–4,4 s |
| **BMW 750d xDrive** | 3.0 R6 Quad-Turbo Diesel | **400** | 4,6 s |
| BMW 750i xDrive | 4.4 V8 Biturbo | 450 / 530 | 4,0–4,5 s |
| Mercedes S 400 d | 3.0 R6 Diesel | 340 | 5,2–5,4 s |

Die Werte sind Beispieldaten für diese Vorschau.[^daten] **Beim BMW 750i ändert das Modelljahr den Vergleich:** Ein einzelner Leistungswert reicht dafür nicht aus.

### Ausstattung im Detail

| Fahrzeug | Anforderung | Head-up-Display | Worauf achten? |
| :--- | :--- | :--- | :--- |
| **BMW 750d xDrive** (G11/G12, 2016–2020) | Ja, **400 PS** | Je nach Ausstattung vorhanden | Ausstattungsliste prüfen und Funktion bei der Probefahrt testen. |
| Mercedes S 400 d 4MATIC (W222 Modellpflege) | Knapp darunter: *340 PS* | Als Option erhältlich | [Checkliste ansehen](https://example.com/ausstattung), insbesondere die Ausstattung des konkreten Fahrzeugs. |
| Audi A8 60 TDI quattro | Ja, 435 PS | Fahrzeugabhängig | Den Eintrag \`head_up_display\` mit der tatsächlichen Ausstattung vergleichen. |

### So würde ich die Auswahl eingrenzen

1. **Das konkrete Fahrzeug prüfen.** Kilometerstand, Wartung und Ausstattung gehören zusammen.
2. Eine kurze Probefahrt vorbereiten:
   - Kaltstart und ruhigen Motorlauf beobachten.
   - Head-up-Display bei Tageslicht testen.
   - Die Anzeige aus der eigenen Sitzposition einstellen.
3. Die offenen Punkte vor dem Kauf schriftlich klären.

> Diese Vorschau prüft, ob ein längerer Hinweis als zusammenhängender Text lesbar bleibt und sich gut vom übrigen Antworttext absetzt.

### Beispiel für eine gespeicherte Auswahl

Inline-Code wie \`preferredModel\` soll sich ruhig in einen Satz einfügen. Ein Codeblock braucht genug Raum für Einrückungen und längere Zeilen:

\`\`\`typescript
const shortlist = cars.filter((car) => {
  return car.fuel === "diesel" && car.allWheelDrive && car.powerPs > 350;
});

const selection = { model: "BMW 750d xDrive", requiredFeatures: ["all-wheel drive", "head-up display"], maximumPrice: 55000 };
\`\`\`

Ein langer Link darf die Antwortspalte nicht verbreitern: https://example.com/fahrzeuge/vergleich/ausstattungsmerkmale-und-modelljahre-fuer-die-naechste-probefahrt

---

**Nächster Schritt:** Zwei oder drei konkrete Angebote mit identischen Kriterien vergleichen. Dann werden Unterschiede bei Ausstattung und Zustand deutlich.

[^daten]: Die Vorschau verwendet feste Beispieldaten. Die Angaben sind keine Kaufberatung.
`;

export const wideMarkdown = `## Breite Tabelle

Die Tabelle darf horizontal scrollen, während der Text in der verfügbaren Spalte bleibt. Jede Information bleibt erreichbar.

| Fahrzeug | Motor | Leistung | Drehmoment | Antrieb | Baujahre | Ausstattung | Hinweis |
| :--- | :--- | ---: | ---: | :--- | :--- | :--- | :--- |
| BMW 750d xDrive | R6 Diesel | 400 PS | 760 Nm | Allrad | 2016–2020 | Head-up-Display, Komfortsitze | Längere Beschreibung mit **wichtigen Details** und einem [Link](https://example.com/bmw). |
| Mercedes S 400 d | R6 Diesel | 340 PS | 700 Nm | 4MATIC | 2017–2020 | Fahrzeugabhängig | Die vollständige Ausstattungsliste prüfen. |
| Audi A8 60 TDI | V8 Diesel | 435 PS | 900 Nm | quattro | 2020 | Luftfederung | Modelljahr und Ausstattung abgleichen. |

### Der nächste Abschnitt

Auch nach einer breiten Tabelle bleibt der Abstand zum nächsten Absatz konsistent.
`;
