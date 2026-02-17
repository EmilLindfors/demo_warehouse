# Kjerne-mellommodeller: Designbegrunnelse

Dette dokumentet forklarer hvordan `int_reservoir_enriched` og `int_precipitation_weekly` er bygget opp, designvalgene bak dem, og hvordan de betjener marts-laget.

## Oversikt over dataarkitekturen

```
Sources (raw)       Staging (clean)        Intermediate (enrich)     Marts (serve)
─────────────       ──────────────         ────────────────────      ────────────
raw_nve.            stg_reservoir_stats ──┐
 reservoir_stats                          ├─► int_reservoir_enriched ──► fct_reservoir_weekly
raw_nve.            stg_reservoir_        │                               │
 reservoir_         min_max_median  ──────┘                               ├─► rpt_reservoir_trends
 min_max_median                                                           ├─► rpt_biggest_changes
                                                                          │
raw_frost.          stg_precipitation ────► int_precipitation_weekly ──────┤
 precipitation                                                            ├─► rpt_precipitation_trends
                                                                          └─► rpt_biggest_changes
```

Begge mellommodellene deler samme grain: **én rad per el-område (NO1–NO5) per ISO-uke**. Denne samkjøringen er bevisst — NVE publiserer magasindata ukentlig, så nedbørsdata må aggregeres opp for å matche.

---

## `int_reservoir_enriched`

### Hva den gjør

Tar de rensede ukentlige magasinobservasjonene og beriker hver rad med to ekstra dimensjoner av kontekst:

1. **Historiske bånd** — Hvordan er denne ukens fyllingsgrad sammenlignet med historisk min, maks og median for samme ISO-uke?
2. **År-over-år** — Hva var fyllingsgraden i samme uke i fjor?

### Hvordan den fungerer (steg for steg)

```sql
-- CTE 1: reservoir
-- Pulls cleaned weekly reservoir data (one row per area per week)
-- Columns: el_area, area_number, observation_date, iso_year, iso_week,
--          fill_pct, fill_pct_prev_week, fill_pct_change, capacity_twh, fill_twh

-- CTE 2: historical
-- Pre-computed min/max/median fill percentages per area per ISO week
-- from NVE's historical dataset (not computed by us — NVE provides these bands)
-- Columns: el_area, iso_week, min_fill_pct, max_fill_pct, median_fill_pct

-- CTE 3: prev_year
-- Self-join trick: shifts iso_year forward by 1 so that last year's data
-- can be joined to this year's rows on (el_area, iso_year, iso_week)
-- Columns: el_area, compare_year (= iso_year + 1), iso_week, fill_pct_prev_year

-- CTE 4: enriched
-- Joins all three together and computes derived columns
```

### Viktige designvalg

**1. Self-join for år-over-år — ISO-år+1-trikset**

Målet: for hver rad (f.eks. NO1, 2025, uke 10), legg ved fjorårets fyllingsgrad (NO1, 2024, uke 10).

Den naive tilnærmingen ville vært å joine `reservoir` med seg selv med `r.iso_year = r2.iso_year + 1`. Det fungerer, men krever aliasing av den samme store tabellen to ganger. I stedet bygger vi en liten CTE som forskyvér året på forhånd:

```sql
prev_year as (
    select
        el_area,
        iso_year + 1  as compare_year,   -- ← trikset: 2024 blir 2025
        iso_week,
        fill_pct      as fill_pct_prev_year
    from reservoir
)
```

Her er hva som skjer med konkrete data. `reservoir`-tabellen inneholder:

| el_area | iso_year | iso_week | fill_pct |
|---------|----------|----------|----------|
| NO1     | 2024     | 10       | 62.5     |
| NO1     | 2025     | 10       | 58.3     |

`prev_year`-CTE-en transformerer 2024-raden til:

| el_area | compare_year | iso_week | fill_pct_prev_year |
|---------|--------------|----------|--------------------|
| NO1     | **2025**     | 10       | 62.5               |

Nå joiner vi med `r.iso_year = py.compare_year`:

```sql
from reservoir r
left join prev_year py
    on  r.el_area  = py.el_area
    and r.iso_year = py.compare_year   -- 2025 = 2025 ✓
    and r.iso_week = py.iso_week       -- 10 = 10 ✓
```

Raden for 2025-uke-10 plukker opp 2024s 62,5 % som sin `fill_pct_prev_year`. Join-betingelsen er naturlig å lese: «match meg med raden som *ønsker å bli sammenlignet med* mitt år.»

**Hvorfor dette fungerer bedre enn `LAG()`:**

- `LAG(fill_pct, 52)` ville antatt nøyaktig 52 uker per år, men ISO-år har enten 52 eller 53 uker. År 2020 hadde 53 uker, så `LAG(..., 52)` i 2021 ville vært forskjøvet med én uke for hele året.
- `LAG()` krever også en perfekt ordnet, sammenhengende sekvens. Hvis noen uker mangler i dataene, forskyves alle etterfølgende LAG-verdier.
- Join-tilnærmingen matcher på den *semantiske nøkkelen* (samme område, samme ukenummer) i stedet for på radposisjon, og håndterer dermed hull og 52/53-ukers år korrekt. Hvis fjoråret ikke hadde uke 53, returnerer LEFT join NULL — som er det korrekte svaret, ikke en feilplassert verdi fra en annen uke.

**2. Historiske bånd kommer fra NVE, ikke fra våre egne data**

Modellen `stg_reservoir_min_max_median` eksponerer NVEs forhåndsberegnede historisk statistikk. Vi valgte å bruke myndighetenes egne bånd fremfor å beregne egne persentiler fordi:
- NVE har en lengre historisk serie enn det vi henter inn
- Det unngår avvik mellom våre analyser og den offisielle referansen
- Joinen er kun på `iso_week` (uten år), siden dette er aggregater på tvers av år

**3. `fill_status`-klassifiseringen**

```sql
case
    when fill_pct <= hist_min_fill_pct then 'historic_low'
    when fill_pct < hist_median_fill_pct - 10 then 'below_normal'
    when fill_pct <= hist_median_fill_pct + 10 then 'normal'
    when fill_pct < hist_max_fill_pct then 'above_normal'
    else 'historic_high'
end
```

Dette gjør en kontinuerlig metrikk om til en kategorisk etikett ved å bruke et symmetrisk 10-prosentpoengs bånd rundt medianen. Tersklene er bevisst enkle:
- «Normal»-båndet er median +/- 10 pp — bredt nok til å unngå støyende svingninger mellom kategorier
- Ekstremer defineres ved å overstige NVEs historiske min/maks, som per definisjon er sjeldne
- Denne klassifiseringen beregnes én gang her og gjenbrukes overalt nedstrøms (faktatabell, rapporter)

**4. `fill_pct_vs_median` som en egen kolonne**

I stedet for kun å eksponere den kategoriske `fill_status`, eksponerer vi også det rå numeriske avviket fra medianen (`fill_pct - hist_median_fill_pct`). Dette gir nedstrøms brukere fleksibilitet: rapporter kan bruke etiketten for fargekoding og tallet for diagrammer/sortering.

**5. LEFT joins gjennomgående**

Både historical- og prev_year-joinene er LEFT joins fordi:
- Det første året med data har ikke noe foregående år å sammenligne med
- Noen ISO-uker kan mangle fra det historiske datasettet
- Vi ønsker aldri å miste en magasinobservasjon bare fordi kontekstdata er utilgjengelig

---

## `int_precipitation_weekly`

### Hva den gjør

Aggregerer daglige stasjonsbaserte nedbørsmålinger til ukentlige områdeoppsummeringer. Dette er en tostegs-opprulling:

1. **Stasjon-ukentlig**: Summer hver stasjons daglige avlesninger til en uketotal
2. **Område-ukentlig**: Beregn gjennomsnittet på tvers av alle stasjoner i samme el-område

### Hvordan den fungerer (steg for steg)

```sql
-- CTE 1: daily
-- Cleaned daily precipitation from Frost API
-- One row per station per day
-- Columns: station_id, el_area, station_name, observation_date,
--          precipitation_mm, quality_code, is_verified

-- CTE 2: station_weekly
-- Groups by (el_area, station_id, iso_year, iso_week) to produce:
--   - weekly_precipitation_mm: total rain at that station that week
--   - avg/max daily precipitation, rainy day count
--   - days_with_data: data completeness indicator
-- IMPORTANT: Filters to only verified observations (is_verified = true)

-- CTE 3: area_weekly
-- Groups by (el_area, iso_year, iso_week) and AVERAGES across stations:
--   - Takes avg(weekly_precipitation_mm) as the representative area value
--   - Also captures spread: min/max station totals, station count
```

### Viktige designvalg

**1. Tostegs aggregering (stasjon -> område) i stedet for én**

Vi kunne gått rett fra daglige observasjoner til område-ukentlig i én enkelt GROUP BY. Men tostegs-tilnærmingen er bedre fordi:
- **Representativitet**: Å ta gjennomsnittet av stasjonstotaler (i stedet for gjennomsnittet av alle daglige avlesninger) gir lik vekt til hver stasjon. En stasjon med 5 dager med data får ikke mindre vekt enn en med 7 dager.
- **Transparens**: Den mellomliggende `station_weekly`-CTE-en produserer metrikker som `days_with_data` som lar oss vurdere datakvaliteten på stasjonsnivå før vi beregner gjennomsnitt.

**2. AVG (ikke SUM) på tvers av stasjoner**

Områdenivåets `weekly_precipitation_mm` er **gjennomsnittet** av alle stasjonenes uketotaler, ikke summen. Dette er fordi:
- Ulike områder har ulikt antall værstasjoner (NO1 har flere enn NO5)
- Summering ville fått områder med flere stasjoner til å fremstå som om de har mer nedbør
- Vi ønsker en representativ nedbørsverdi per område, ikke en total på tvers av instrumenter

**3. Spredningsmål i tillegg til gjennomsnittet**

Vi eksponerer `min_station_weekly_mm` og `max_station_weekly_mm` ved siden av gjennomsnittet. Dette fanger opp variasjon innad i et område — hvis én stasjon målte 80 mm og en annen 5 mm i samme uke, ville gjennomsnittet alene vært misvisende. Nedstrøms brukere kan bruke spredningen for å flagge uker med høy romlig variabilitet.

**4. Kun verifiserte data (`is_verified = true`)**

Frost-APIet gir en `quality_code` for hver observasjon. Staging mapper `quality_code = 0` til `is_verified = true`. Nedbørsmodellen filtrerer til kun verifiserte data i det første aggregeringssteget. Dette er en tidlig, aggressiv kvalitetsport — vi foretrekker manglende data fremfor upålitelige data, spesielt ettersom nedbørsdata brukes i analytiske rapporter om værets påvirkning på magasiner.

**5. `station_count` og `avg_days_with_data` som kvalitetsindikatorer**

Disse er ikke analytiske mål — de er metadata som nedstrøms brukere kan benytte for å:
- Filtrere ut uker med for få rapporterende stasjoner
- Flagge område-uker der stasjoner hadde ufullstendig dekning (avg_days_with_data < 7)
- Legge til forbehold i rapporter når datadekningen er lav

---

## Mart-modeller: Dypdykk

Marts-laget følger en dimensjonsmodell: én dimensjonstabell, én faktatabell og tre rapportklare modeller. Alle marts er materialisert som tabeller (ikke views) for spørringsytelse.

---

### `dim_el_area` — Dimensjonstabellen

```sql
select
    el_area,
    area_number,
    area_name,
    area_description,
    weather_station_count
from {{ ref('int_dimensions') }}
```

Dette er den enkleste modellen i lageret — en direkte gjennomkobling fra `int_dimensions`. Den finnes som en separat mart-modell (i stedet for at rapporter spør `int_dimensions` direkte) av to grunner:

1. **Lagdisiplin**: Marts bør kun referere til andre marts eller mellommodeller, og rapporter bør joine mot dimensjoner i mart-laget. Hvis vi hoppet over denne modellen, ville rapporter strekke seg på tvers av lag.
2. **Materialiseringsgrense**: `int_dimensions` er en view; `dim_el_area` er en tabell. Det betyr at områdeoppslag i rapportspørringer treffer en liten materialisert tabell i stedet for å kjøre de oppstrøms joinene på nytt hver gang.

Kolonnen `weather_station_count` er dynamisk — den beregnes fra de faktiske nedbørsdataene, ikke hardkodet. Hvis vi legger til værstasjoner i inntaket, oppdateres denne tellingen automatisk ved neste `dbt build`.

---

### `fct_reservoir_weekly` — Faktatabellen

```sql
select
    -- keys
    el_area, observation_date, iso_year, iso_week,
    -- current measures
    fill_pct, fill_twh, capacity_twh, fill_pct_change,
    -- historical context
    hist_min_fill_pct, hist_max_fill_pct, hist_median_fill_pct,
    fill_pct_vs_median, fill_status,
    -- year-over-year
    fill_pct_prev_year, fill_pct_yoy_change
from {{ ref('int_reservoir_enriched') }}
```

**Grain**: én rad per el-område per ISO-uke.

Dette er et kuratert kolonneutvalg fra `int_reservoir_enriched`. Den dropper bevisst to kolonner som finnes i mellommodellen:

- `area_number` — et dimensjonsattributt, ikke et fakta; brukere bør hente det fra `dim_el_area`
- `fill_pct_prev_week` — overflødig med `fill_pct_change` (som er `fill_pct - fill_pct_prev_week`); å beholde begge ville invitert til inkonsistens

**Hvorfor ha en separat faktatabell hvis det bare er et kolonneutvalg?**

Faktatabellen er **kontrakten** mellom mellomlaget og rapportlaget. Den definerer nøyaktig hvilke kolonner som er stabile, testede og trygge å bygge på. Mellommodellen er en implementasjonsdetalj som kan refaktoreres (f.eks. splitte historical-joinen inn i en separat CTE); faktatabelens grensesnitt forblir det samme.

Dette er også grunnen til at `fct_reservoir_weekly` har `not_null`-tester på `el_area` og `observation_date` — disse er kolonnene som definerer grain, og å teste dem her (ved kontraktgrensen) fanger opp problemer før de forplanter seg til rapporter.

---

### `rpt_reservoir_trends` — Magasinrapportmodellen

Dette er den rikeste mart-modellen. Den joiner faktatabellen med dimensjonen og legger til tre window functions.

**Steg 1: Join for lesbare navn**

```sql
from reservoir r
left join area_info a
    on r.el_area = a.el_area
```

Alle rapportmodeller gjør denne joinen. Vi denormaliserer områdenavnet inn i rapporten slik at BI-verktøy og dashbord ikke trenger å gjøre egne joins — rapporten er selvstendig og spørringsferdig.

**Steg 2: Hittil-i-år løpende min og maks**

```sql
min(r.fill_pct) over (
    partition by r.el_area, r.iso_year
    order by r.iso_week
    rows between unbounded preceding and current row
) as ytd_min_fill_pct,

max(r.fill_pct) over (
    partition by r.el_area, r.iso_year
    order by r.iso_week
    rows between unbounded preceding and current row
) as ytd_max_fill_pct
```

Disse svarer på «hva er det laveste/høyeste dette området har vært *hittil i år*?» Vinduet er partisjonert etter område og år, sortert etter uke, og ser på alle foregående rader opp til den nåværende (`unbounded preceding to current row`).

Brukstilfelle: innen uke 40, hvis `ytd_min_fill_pct` er 25 %, vet du at magasinene falt til 25 % på et tidspunkt tidligere i året, selv om de nå er på 70 %. Dette er nyttig for risikovurdering — hvor nær kritiske nivåer kom vi?

**Steg 3: 12-ukers glidende gjennomsnitt**

```sql
round(avg(r.fill_pct) over (
    partition by r.el_area
    order by r.observation_date
    rows between 11 preceding and current row
), 2) as fill_pct_12w_avg
```

Dette jevner ut ukentlig støy for å avdekke den underliggende sesongtrenden. Viktige detaljer:

- **12 uker (~3 måneder)**: Langt nok til å dempe værrelaterte topper, kort nok til å fortsatt vise sesongmønstre (vårsmeltingen, høstfylling)
- **Partisjonert kun etter område** (ingen årspartisjon): Vinduet krysser årsgrenser, så gjennomsnittet i januar inkluderer data fra foregående november/desember. Dette er tilsiktet — sesongssyklusen nullstilles ikke 1. januar.
- **`rows between 11 preceding and current row`**: 11 + nåværende = 12 rader. For de første 11 radene i hvert områdes historikk beregnes gjennomsnittet over færre rader (det venter ikke på et fullt vindu).
- **Sortert etter `observation_date`** (ikke etter år+uke): Datoer sorterer korrekt på tvers av årsgrenser; sortering etter `(iso_year, iso_week)` ville også fungert, men `observation_date` er mer eksplisitt.

**Hvorfor disse window functions lever her og ikke i mellomlaget:**

Dette er presentasjonsspesifikke analyser. Hittil-i-år-ekstremer og glidende gjennomsnitt er nyttige for diagrammer og dashbord, men de trengs ikke av andre modeller. Ved å holde dem utenfor `int_reservoir_enriched` forblir den modellen fokusert på kjerneoppgaven (beriking), og andre brukere (som `rpt_biggest_changes`) slipper kostnaden ved å beregne ubrukte window functions.

---

### `rpt_precipitation_trends` — Nedbørsrapportmodellen

Speiler strukturen til `rpt_reservoir_trends`, men for nedbørsdata. Den har tre tillegg utover `int_precipitation_weekly`:

**1. År-over-år-sammenligning (samme self-join-triks)**

```sql
prev_year as (
    select
        el_area,
        iso_year + 1  as compare_year,
        iso_week,
        weekly_precipitation_mm as precip_mm_prev_year
    from weekly
)
-- then joined: w.iso_year = py.compare_year AND w.iso_week = py.iso_week
```

Denne bruker nøyaktig samme `iso_year + 1`-mønster som magasinmodellen. Forskjellen er at for nedbør lever år-over-år-beregningen **her i mart-modellen**, ikke i mellomlaget.

Hvorfor asymmetrien? I magasin-pipelinen er år-over-år en kjerneberiking — `fill_pct_yoy_change` mater inn i faktatabellen og kan påvirke `fill_status`-logikken. For nedbør er år-over-år utelukkende et rapporteringsbehov: ingen annen modell trenger «fjorårets nedbør» som input. Derfor blir den i rapportmodellen, slik at `int_precipitation_weekly` kan fokusere på aggregeringsjobben.

**2. Hittil-i-år kumulativ nedbør**

```sql
round(sum(w.weekly_precipitation_mm) over (
    partition by w.el_area, w.iso_year
    order by w.iso_week
    rows between unbounded preceding and current row
), 1) as ytd_cumulative_precipitation_mm
```

En løpende sum (ikke et løpende gjennomsnitt som i magasinmodellen). Denne svarer på «hvor mye nedbør har falt i dette området hittil i år?» — nyttig for å sammenligne total årlig nedbør på tvers av år.

Merk at partisjonen inkluderer `iso_year` — i motsetning til 12-ukersgjennomsnittet, *bør* den kumulative totalen nullstilles 1. januar fordi den sporer et årsbudsjett.

**3. 12-ukers glidende gjennomsnitt**

```sql
round(avg(w.weekly_precipitation_mm) over (
    partition by w.el_area
    order by w.iso_year, w.iso_week
    rows between 11 preceding and current row
), 1) as precip_12w_avg_mm
```

Samme logikk som magasinets glidende gjennomsnitt: ingen årspartisjon (krysser årsgrenser), 12-raders vindu. Sortert etter `(iso_year, iso_week)` her i stedet for etter dato — begge tilnærmingene fungerer, men denne modellen har ingen enkelt `observation_date`-kolonne (den har `week_start_date` og `week_end_date`), så sortering etter år+uke er renere.

---

### `rpt_biggest_changes` — Ekstremer på tvers av domener

Dette er den eneste modellen som bringer magasin- og nedbørsdata sammen. Den lager en union av to sett med rader til én enkelt tabell med en `change_type`-diskriminator.

**Magasinsiden:**

```sql
reservoir_changes as (
    select
        r.el_area, a.area_name,
        r.iso_year, r.iso_week, r.observation_date,
        r.fill_pct, r.fill_pct_change,
        'reservoir_fill_change' as change_type
    from reservoir r
    left join area_info a on r.el_area = a.el_area
    where r.fill_pct_change is not null
)
```

**Nedbørssiden:**

```sql
precip_peaks as (
    select
        p.el_area, a.area_name,
        p.iso_year, p.iso_week, p.week_start_date as observation_date,
        cast(null as double) as fill_pct,
        cast(null as double) as fill_pct_change,
        p.weekly_precipitation_mm,
        'precipitation_peak' as change_type
    from precip_weekly p
    left join area_info a on p.el_area = a.el_area
)
```

**Designvalg:**

**1. UNION ALL med NULL-utfylling, ikke en JOIN**

Dette er to ulike *typer* hendelser (magasinendringer vs. nedbørstopper), ikke to attributter av samme hendelse. En join ville implisert et 1:1-forhold mellom magasin- og nedbørsuker, noe som ikke alltid stemmer (datahull). Unionen produserer en lang-format-tabell der hver rad er én type hendelse, med NULL-verdier for kolonnene som ikke gjelder.

`change_type`-kolonnen (`'reservoir_fill_change'` eller `'precipitation_peak'`) fungerer som en diskriminator slik at brukere kan filtrere eller pivotere.

**2. Ingen rangering i selve modellen**

Til tross for at modellen heter «biggest changes», bruker den faktisk ikke `ROW_NUMBER()` eller `RANK()` for å velge en topp N. Den leverer rådataene; rangeringen overlates til det konsumerende BI-verktøyet eller spørringen. Dette er bevisst:

- Ulike brukere kan ønske forskjellige topp-N (topp 10? topp 20? topp 5 per år?)
- Rangeringskriterier kan variere (største absolutte endring? kun største fall? kun største økning?)
- Ved å holde modellen urangert betjener én enkelt tabell alle disse brukstilfellene

**3. `cast(null as double)` for typesamsvar**

UNION ALL krever samsvarende kolonnetyper. Nedbørsrader har ingen `fill_pct` eller `fill_pct_change`, og magasinrader har ingen `weekly_precipitation_mm`. I stedet for å utelate disse kolonnene fyller vi med typede NULL-verdier slik at union-skjemaet er konsistent. Dette er et standardmønster for heterogene unioner — alternativet (separate tabeller) ville tvunget brukere til å spørre to tabeller og slå sammen resultatene selv.

**4. Her lønner grain-samkjøringen seg**

Begge sider av unionen deler `(el_area, iso_year, iso_week)` som sin naturlige nøkkel. Dette er kun mulig fordi `int_precipitation_weekly` bevisst ble rullet opp til ukentlig granularitet i mellomlaget. Hadde nedbørsdata forblitt på daglig granularitet, ville denne unionen ikke vært meningsfull — du ville sammenlignet en uke med magasinendring mot en enkelt dag med nedbør.

---

## Hvordan mart-modellene forholder seg til hverandre

```
                    dim_el_area
                    (area names)
                   ╱     |      ╲
                  ╱      |       ╲
  rpt_reservoir_trends   |   rpt_precipitation_trends
         ↑               |              ↑
  fct_reservoir_weekly    |   int_precipitation_weekly
                  ╲      |       ╱
                   ╲     |      ╱
                  rpt_biggest_changes
```

- `dim_el_area` joines av alle tre rapportmodellene for områdenavn — det er den delte dimensjonen
- `fct_reservoir_weekly` er den eneste sannhetskilden for magasindata i marts; både `rpt_reservoir_trends` og `rpt_biggest_changes` leser fra den (aldri fra mellomlaget direkte)
- `int_precipitation_weekly` konsumeres direkte av marts-laget (det finnes ingen `fct_precipitation_weekly`) fordi nedbørsdata ikke trenger samme grad av beriking som magasiner — mellommodellen er allerede på riktig grain og detaljnivå
- `rpt_biggest_changes` er kryssdomene-modellen som knytter de to datastrømmene sammen

---

## Oppsummering av designprinsipper

| Prinsipp | Hvordan det er anvendt |
|----------|----------------------|
| **Samkjør grain tidlig** | Nedbør rulles opp til ukentlig i mellomlaget slik at det matcher magasindata |
| **Berik én gang, bruk mange ganger** | Historiske bånd og år-over-år beregnes én gang i `int_reservoir_enriched`, gjenbrukes på tvers av alle magasin-marts |
| **Skill ansvarsområder etter lag** | Kjerneberiking i mellomlaget; presentasjonsanalyser (window functions) i marts |
| **Foretrekk LEFT joins** | Mist aldri observasjoner på grunn av manglende kontekstdata |
| **Eksponer både etiketter og tall** | `fill_status` (kategorisk) + `fill_pct_vs_median` (numerisk) for fleksibilitet |
| **Kvalitetsporter tidlig** | Nedbør filtreres til kun verifiserte data før enhver aggregering |
| **Transparent aggregering** | Tostegs nedbørsopprulling med kvalitetsmetadata (stasjonsantall, dager med data) |
