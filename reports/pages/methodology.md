# Visualiseringsmetodikk

Forslag til visualisering og begrunnelse for de analytiske valgene i DyrWatt-rapportene.

## Hvorfor Evidence.dev?

Evidence.dev gir interaktive web-dashbord som er tilgjengelige fra nettleser — ingen
desktop-programvare kreves. Rapporter oppdateres automatisk når underliggende data endres,
og kildekoden er versjonskontrollert i Git for full sporbarhet.

## Dashbord-struktur

Rapportene følger et **oversikt-først → detalj-drill-down**-mønster:

1. **Oversikt** (`index`) — nøkkeltall, kombinert fyllingsgrad/nedbør, og trender for alle el-områder
2. **Magasin-detaljer** (`reservoir`) — dybdeanalyse av fyllingsgrad per område
3. **Nedbør-detaljer** (`precipitation`) — dybdeanalyse av nedbør per område
4. **Største endringer** (`biggest-changes`) — rangerte perioder med størst endring

## Diagramtyper og begrunnelse

| Diagramtype | Bruksområde | Begrunnelse |
|---|---|---|
| **Linjediagram** | Fyllingsgrad over tid, kumulativ nedbør | Viser kontinuerlige trender og sesongmønstre tydelig |
| **Stolpediagram** | Ukentlig nedbør, ukentlig endring, YoY delta | Egnet for diskrete perioder der hver verdi er uavhengig |
| **Arealdiagram** | Historisk min/maks-bånd | Fylt område viser normalområdet; avvik synes umiddelbart |
| **Datatabell** | Rangerte endringer, statusoversikt | Sortérbar og søkbar — ideelt for presis oppslag |
| **BigValue (KPI)** | Min, maks, gjennomsnitt | Gir executive-oversikt uten å kreve diagramtolkning |

## Analytiske teknikker

### 12-ukers glidende snitt
Jevner ut ukentlig støy over et kvartal slik at sesongmessige trender blir synlige. Brukes på
både fyllingsgrad og nedbør.

### Kumulativ nedbør (year-to-date)
Summerer nedbør fra årets start. Gjør det mulig å sammenligne sesongutvikling
mellom år uavhengig av enkeltukevariasjon.

### Multi-år ISO-uke-overlay
Legger ulike år oppå hverandre med ISO-uke som x-akse. Gir eple-til-eple-
sammenligning av sesongforløp uten datoforskyvning.

### Historisk min/maks/median-bånd
Viser normalområdet basert på hele historikken. Gjeldende verdi plottes innenfor
båndet slik at det er lett å se om nivået er uvanlig høyt eller lavt.

### Fyllingsgrad-klassifisering
Automatisk kategorisering av fyllingsgrad i nivåer fra `historic_low` til
`historic_high` basert på historisk fordeling. Synliggjøres i statuskolonnen.

## Interaktivitet

Alle detaljsider har **dropdown-filtre** for:

- **El-område** — velg blant NO1–NO5
- **Fra år / Til år** — begrens tidsperioden

Filtrene oppdaterer alle diagrammer og tabeller på siden i sanntid.

## Dimensjonsmodell

Underliggende data er organisert i et **stjerneskjema** med to rapporteringstabeller:

- `rpt_reservoir_trends` — ukentlig fyllingsgrad med historiske nøkkeltall
- `rpt_precipitation_trends` — ukentlig nedbør med kumulative summer

Begge deler `el_area`, `iso_year` og `iso_week` som felles dimensjoner, noe som
muliggjør sammenstilling av magasin- og nedbørsdata i kombinert analyse.
