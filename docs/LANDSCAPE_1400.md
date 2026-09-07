# 600 More: Materials You Can Hold, and a World That Breathes

**Status:** v1.0 · **Date:** 2026-09-06 · **Items 801–1400**
**Continues:** `LANDSCAPE_200.md` (1–200), `LANDSCAPE_800.md` (201–800)

---

## Holistic state, measured

Since the last document an enormous amount has landed. Verified by build and
selftest, not assumed:

```
routing rerouted : 157 cells changed downstream · signature CHANGED
depression fill  : lake cells 259475 · mean fill depth 1.007 m
biosphere 1-day  : plants 12000 · rain 0.004 · moisture 0.176 · N 143527
```

**The one-week test from `LANDSCAPE_200.md` now passes.** Digging a trench
reroutes drainage. The 97 %-of-the-drum lake bug is fixed — finite-volume lakes
now sit at a mean fill depth of one metre instead of a hundred and seventy-nine.

Present in `sim/src/`: `flow`, `lakes`, `soil`, `weather`, `plant`, `biome`,
`biosphere`, `erosion`, `material`, `paint`, `persist`, `sph`. That is most of
sections B, C, D, F, G and H of the first document, implemented.

### The hole

**Nothing you dig produces anything.** `material_at()` knows the rock is
sandstone and `dig_scale()` makes it slow to cut, and then the material ceases to
exist. There is no inventory, no mass, no processing, no craft, no consequence.

In a **closed** habitat that is not a missing feature, it is a missing *premise*.
Every gram of iron in the drum is already there. You cannot import, you can only
move mass between pools — and that constraint is the most interesting economy a
game can have. Section X is the largest here for that reason.

---

## Adopted from ORRERY, third pass

The ecology spine, which we have been approximating by hand.

| ORRERY | What we take |
|---|---|
| `ecology.js` — **Miami model NPP** from temperature and precipitation | A cited empirical productivity model instead of our invented biomass curve. |
| `evolve.js` + `genome.js` — traits, body-plan expression, speciation, phylogeny, horizontal gene transfer, morph penalty | An actual evolution layer, and a genome that expresses rather than a trait vector that lerps. |
| `kleiberDensity` | Population density derived from body mass by **Kleiber's law**, so fauna numbers come from physics rather than from tuning. |
| `trophicField.js` — producers / grazers / hunters / detritus as fields, discrete carcasses, and a **predation-pressure "landscape of fear"** prey read when fleeing | The whole food web, and the single best behavioural idea in their codebase. |
| `shannonDiversity` | Diversity as a measured index, not a vibe. |
| `sensory.js` — `sensoryEnvAt`, `viableBands` | Creatures perceive the world through a modelled sensory environment. |
| `catalogue.js` — auto-generated from a script, never hand-edited | Species and material catalogues compiled from data. |
| `lifeGuide.js` | The in-game field guide, driven by the same data. |

---

## X. The material economy — what you grab and what you make (801–890)

*The closed system's gift: extraction is permanent, mass is conserved, and every
recipe is a transfer between pools you can audit.*

### Extraction yields

801. **A dig must return mass.** `dig()` already computes the material and the
     effective radius; return `(material_id, cubic_metres)` from the volume the
     CSG stroke actually removed rather than discarding it.
802. Compute removed volume properly by integrating the stroke against the prior
     density field, not by assuming a full sphere — half a brush against a cliff
     yields half.
803. Bulk density per material, so volume becomes mass. Basalt is three tonnes a
     cubic metre; you will not be pocketing it.
804. A **bulking factor** on excavation (~1.25 for rock): broken material occupies
     more than it did in place, which is why spoil heaps are bigger than holes.
805. Mixed yields: a stroke crossing a strata boundary returns a *mixture*, and
     mixtures need separating (item 823).
806. Ore grade as a continuous value, not a boolean — a vein has a percentage,
     and low grade is a real decision about whether to bother.
807. Grade falling off from a vein's core, so following it well matters.
808. **Depletion**: a vein is finite. Track extracted mass per vein and let it
     run out permanently.
809. Habitat-wide reserves as a known, finite, displayed number. The colony
     should know how much iron exists.
810. Prospecting yields information, not ore: an assay gives grade and extent.
811. Surface indications — stained rock, float in stream beds (§N item 262) — as
     the cheap way to find veins before the expensive way.
812. Overburden: worthless material above the good stuff that must still be
     moved and put somewhere.
813. Harvest yields for vegetation from the plant model's actual biomass pools
     (`plant.rs` already has root/leaf/stem/reproductive) rather than a lookup.
814. Timber volume from stem biomass via the allometry in §Q item 412.
815. Fibre, resin, latex and tannin as secondary products of specific species.
816. Fauna yields — meat, hide, bone, sinew, fat — from body mass.
817. Water as a carried material with real weight, which is why you build
     channels instead.
818. Ice as a harvestable material near the cold endcaps.
819. Air itself as a resource in the ledger — sealing a cave and filling it costs
     atmosphere the drum cannot replace.

### Carrying and moving

820. Inventory with **mass and volume**, both binding, so you cannot carry a
     tonne of basalt in a satchel.
821. Encumbrance affecting movement speed and jump height, using the spin-gravity
     value we already compute.
822. Dropping and stockpiling material as physical objects in the world, visible
     as growing heaps.
823. Carts, barrows and sledges with capacity and terrain restrictions — a cart
     needs a graded path, which is a reason to terrace.
824. Rafts and log driving on rivers of sufficient discharge (§O item 343).
825. Cable ways across valleys, exploiting the terrain rather than fighting it.
826. Pipes and launders for water and slurry, following gradient.
827. Pneumatic transport as a late-game option that costs power.
828. Draught animals (§S item 584) as a transport tier between back and machine.
829. Transport cost as a real economic pressure that makes *where* you process
     matter as much as *what*.
830. Stockpiles that decay: ore weathers, timber rots, grain spoils.

### Processing chains

831. **Recipes as hot-reloadable data**, never code — inputs, outputs, energy,
     time, byproducts, and required station.
832. Every recipe must **balance mass** against the ledger, enforced by a test
     that fails the build if it does not.
833. Crushing: rock to gravel to sand, each step costing energy and producing
     dust.
834. Screening and sieving to separate mixtures by size.
835. Washing to separate by density, which needs water and returns it dirty
     (§O item 334).
836. Panning and sluicing as the early-game, no-infrastructure version.
837. Froth flotation or its analogue for fine ore, late game.
838. Roasting to drive off volatiles before smelting.
839. **Smelting**: ore plus fuel plus heat gives metal plus slag plus gas. Three
     outputs, and two of them are problems.
840. Fuel from charcoal, which comes from timber, which comes from forests —
     **your metal industry is limited by your forests**, which is the single best
     coupling in the whole economy.
841. Charcoal production in a kiln, with yield well below the input mass.
842. Coke or its analogue if a suitable material exists.
843. **Smelting consumes oxygen and emits carbon dioxide into a sealed
     atmosphere.** Scale up and the colony notices. This is not a penalty
     mechanic; it is arithmetic.
844. Atmospheric scrubbing as a countermeasure that itself costs power.
845. Slag as a real waste stream that must be dumped, and which is inert enough
     to be useful as aggregate.
846. Refining to raise purity, with diminishing returns and rising cost.
847. Alloying two metals to get properties neither has.
848. Casting into moulds made of sand or clay, consumed or reusable.
849. Forging with hammer and anvil, improving grain and strength.
850. Heat treatment — quenching and tempering — as a quality multiplier requiring
     controlled temperature and a water or oil bath.
851. **Clay to ceramic**: dig clay (`material.rs` already has it), form, dry, fire.
     Vessels, pipes, tiles, crucibles.
852. Drying as a stage that depends on the actual humidity field — you cannot
     fire wet clay, and in a wet band it takes longer.
853. **Sand to glass**, needing the highest heat in the game and a flux.
854. Glass to glazing to greenhouses, which raise local temperature and light
     transmission and therefore raise yield. Mining sand makes food. That loop is
     the thesis of this section.
855. Lime from calcite: burn it, and get both mortar and a soil pH amendment
     (§B item 26). One material, two completely different uses.
856. Mortar and concrete from lime, sand and aggregate.
857. Timber to planks, beams and shingles, with waste as offcuts and sawdust.
858. Sawdust to fuel, litter or compost — nothing is waste in a closed system,
     only misplaced.
859. Retting and scutching fibre plants to linen-analogue.
860. Spinning and weaving to cloth, and cloth to clothing and sacks.
861. Cordage from fibre, which everything else needs.
862. Tanning hides with tannin from bark, which costs trees.
863. Rendering fat to tallow, for candles, soap and lubricant.
864. Milling grain to flour with a water wheel (§O item 328) or by hand.
865. Malting, brewing and fermenting as preservation and morale.
866. Pressing oil from seeds, with the cake as animal feed.
867. Composting with a real C:N ratio, temperature and time (§B item 30).
868. **Biochar** from pyrolysis, which locks carbon in the soil and raises its
     water-holding capacity — a genuine closed-loop carbon sink.
869. Ash from any burning, returning potassium to the soil. The K in NPK has to
     come from somewhere.
870. Bone meal for phosphorus, which is the nutrient a closed colony will run
     short of first.
871. Night soil and manure returning nitrogen (§S item 583). Unglamorous and
     mandatory.

### Tools, quality and consequence

872. Tools with material, quality and wear, affecting dig rate against
     `dig_scale()`.
873. Tool breakage and repair, with repair costing less material than replacement.
874. Better tools unlocking harder materials, so the tech ladder is a *material*
     ladder rather than a menu of unlocks.
875. Quality tiers derived from input purity and process control, not from a
     random roll.
876. Wear visible on the tool model, so condition is readable without a UI.
877. Specialised tools that are better at one job and worse at others.
878. Machines as placed stations with power draw, throughput and maintenance.
879. Automation chains that free labour but consume power, trading one scarce
     resource for another.
880. Breakdown and maintenance schedules, so infrastructure is a commitment.
881. **Recycling as mandatory, not virtuous.** Metal is finite; scrap is the
     second-largest ore body in the habitat.
882. Recovery efficiency below one hundred per cent, so each cycle loses a little
     and the long-term trend is real.
883. Landfill and tailings as places you have to choose, which then leach into
     groundwater (§I item 160).
884. Contamination with a long half-life, so a bad decision outlives the person
     who made it.
885. Remediation as a slow, expensive, satisfying late-game project.
886. **A materials ledger view**: every pool, its stock, its flows in and out, and
     its trend. The economic twin of the biome census already in the survey view.
887. Sankey-style flow visualisation of the whole economy, generated from the
     recipe data rather than drawn.
888. A "where did my phosphorus go" query, following item 197's causal chains
     into the economy.
889. Scarcity as the driver of colony politics, since the drum cannot import.
890. Publish the limits: no thermodynamic simulation, recipes as authored mass
     balances, single-step smelting, no trace elements.

## Y. Fields, plots and the work of farming (891–950)

*The soil grid exists. This is what makes standing in a field feel like work.*

891. Plots as **claimed regions of the soil grid**, not as placed objects — a
     field is an area you have improved, and its edges are where you stopped.
892. Field boundaries drawn by the player, snapped to terrain contours if wanted.
893. Clearing as the first real labour: remove vegetation, roots and stones, all
     of which yield material (§X).
894. Stone picking, with the stones piling into walls at the field edge. That is
     how every dry-stone wall on Earth came to exist, and it should be how yours
     does.
895. Root grubbing that leaves the soil disturbed and needing a season.
896. **Ploughing** breaking compaction (§B item 27), burying residue, and exposing
     soil to erosion. Every benefit paired with a cost.
897. Tilth as a soil state produced by working and destroyed by rain.
898. Seedbed preparation gating germination rate.
899. Sowing density as a real decision — dense yields more per area until it
     self-thins (§Q item 418).
900. Row orientation mattering for light interception, and on a cylinder the
     choice is axial or circumferential.
901. Transplanting seedlings raised in a nursery, trading labour for a head start.
902. Thinning as a maintenance action.
903. Weeding, with weeds genuinely competing in the same light and nutrient model.
904. Mulching to suppress weeds, hold moisture and add organic matter.
905. Irrigation scheduling against the real soil moisture field, not a timer.
906. Over-irrigation causing waterlogging and leaching (§C item 62).
907. Drainage works — ditches, tile drains, French drains — as buildable.
908. Salinity accumulation where irrigation water evaporates without flushing.
     The classic way irrigated civilisations destroyed their own soil.
909. Fertilising with real amendments and real nutrient contents.
910. Over-fertilising causing runoff and eutrophication downstream (§O item 335).
911. Liming to correct pH, from the lime you burned in §X item 855.
912. Green manure ploughed in as a nitrogen and organic input.
913. Fallow with a measurable recovery curve.
914. Rotation planning UI showing the nutrient consequence of a sequence.
915. Perennial polyculture as a low-labour, low-yield, high-stability option.
916. Terraces (§K item 188) genuinely reducing erosion and holding water.
917. Contour ploughing with a measurable effect.
918. Windbreaks and shelter belts changing the local wind field (§P item 405).
919. Hedgerows as habitat for pollinators and pest predators (§S item 568), so
     leaving a margin has a mechanical return.
920. Field margins and beetle banks as a deliberate biodiversity investment.
921. Frost pockets in hollows, which the terrain already produces (§N item 267)
     and which make some fields quietly worse.
922. Aspect affecting warmth, so a slope facing the strip differs from one facing
     away.
923. Microclimate mapping so the player can find the good ground rather than
     being told.
924. **Yield variation within a field**, visible as patchiness, driven by the
     underlying soil grid. A uniformly green rectangle is a lie.
925. Harvest as a real operation with time, labour and losses.
926. Threshing, winnowing and cleaning as separable steps.
927. Storage losses to moisture, pests and heat.
928. Seed saved from the crop, with its own genetics (§R item 544).
929. Crop failure with a legible cause, always.
930. Partial failure — a poor year, not a binary.
931. Gleaning and salvage from a failed crop.
932. Livestock grazing crop residue, returning manure.
933. Integrated systems where animals, crops and trees share ground.
934. Aquaculture in ponds the player dug, fed by the river.
935. Paddy-style flooded cultivation where terrain and water allow.
936. Greenhouses from the glass in §X item 854, raising temperature and season
     length locally.
937. Cold frames and cloches as the cheap version.
938. Hydroponics as a power-hungry, soil-free alternative for high-value crops.
939. Mushroom cultivation in caves, on the sawdust from §X item 858.
940. Beekeeping, which pollinates and yields, and which fails if the forage fails.
941. Orchards as a multi-decade investment that outlives the planting decision.
942. Coppice rotation for a sustainable fuel supply (§Q item 446), which is what
     keeps the smelters running.
943. **Labour as a real constraint** once colonists exist: a colony can only work
     so much ground.
944. Seasonal labour peaks that force prioritisation.
945. Tools and machines reducing labour per hectare, at a material and power cost.
946. Field records: what was planted, what was applied, what was got.
947. A farm ledger tying field records to the habitat nutrient budget.
948. Experiment plots for testing a variety or treatment against a control.
949. The agronomy console as the place all of this is read (§B item 34,
     `REQUIREMENTS.md` D2).
950. Publish the limits: no tillage physics, no root architecture, nutrient
     uptake as a simple demand-and-supply match.

## Z. The forest deepened — wood, canopy and the long clock (951–1010)

*`plant.rs` runs carbon allocation. A forest is that model given a century and a
neighbourhood.*

951. Promote the existing carbon model to trees by adding a woody pool that never
     respires away — that single change is what makes a plant a tree.
952. Heartwood and sapwood as separate fractions, with only sapwood conducting.
953. Annual growth increments recorded per tree, giving tree rings for free
     (§Q item 450).
954. Ring width from that year's actual limiting factor, so a core is a readable
     climate record and the player can date a drought.
955. Bark thickness by species, gating fire survival (§Q item 439).
956. Crown shyness — canopies not quite touching — which is subtle, real, and
     beautiful.
957. Apical dominance producing conical young trees that spread when the leader
     is lost.
958. Reaction wood forming when a tree leans, correcting it slowly.
959. Root plate extent from crown extent, which sets windthrow risk (§Q item 435).
960. Buttress roots on large trees in shallow soil.
961. Root grafting between neighbours of the same species, sharing resources.
962. **Mycorrhizal network** as an actual graph between trees, moving carbon from
     the rich to the poor (§Q item 428).
963. Network topology visible in an overlay — a genuinely magical thing to show.
964. Fungal fruiting bodies appearing seasonally where the network is dense.
965. Nurse logs: seedlings establishing preferentially on decaying deadwood.
966. Standing snags persisting for decades as habitat before falling.
967. Cavity formation in old trees, which specific fauna require.
968. Old-growth structure as an emergent property of time plus low disturbance,
     not a biome label.
969. Veteran trees as landmarks with names, prominence and history.
970. A tree's chronicle: when it germinated, what it survived, who planted it.
971. Forest floor light as a computed field, driving everything beneath.
972. Sunflecks moving with the light schedule, briefly raising understory
     photosynthesis — a real and lovely mechanism.
973. Leaf area index per stand as the compact summary the sim actually uses.
974. Canopy interception of rain (§P item 363) feeding back into soil moisture.
975. Evapotranspiration from the canopy raising local humidity (§H item 146).
976. Forest cooling measurable in the temperature field.
977. Windbreak effect of a stand on the banded wind field.
978. Snow retention under canopy, melting later than in the open.
979. Leaf litter depth as a field, insulating soil and suppressing seedlings.
980. Decomposition rate by litter quality and moisture (§B item 28).
981. Nutrient cycling within the stand, so a mature forest is nearly closed.
982. Which means **clearing one releases a nutrient pulse** and then a long
     deficit. Slash-and-burn works once.
983. Species-specific wood properties: density, strength, workability, rot
     resistance, fuel value.
984. Timber grading from those properties plus growth history.
985. Seasoning time by species and thickness (§Q item 449).
986. Construction requiring appropriate timber, so buildings reflect local forest.
987. Charcoal yield varying by wood density — the good fuel wood becomes strategic.
988. Bark for tannin (§X item 862), which costs the tree.
989. Resin tapping as a non-destructive yield.
990. Sap and syrup as a seasonal harvest tied to temperature cycling.
991. Nuts and fruit as a mast year phenomenon — synchronised heavy crops every
     few years, which fauna populations then track with a lag.
992. Coppice stools living for centuries, each cut regrowing faster.
993. Pollarding above browse height where livestock graze beneath.
994. Wood pasture as a genuine land-use type between forest and field.
995. Forest gardens layering canopy, shrub and ground crops.
996. Windthrow gaps as the main natural disturbance, from the wind field.
997. Pit-and-mound microtopography persisting in the terrain for a century
     (§Q item 436).
998. Deadwood volume as a biodiversity index the player can raise deliberately.
999. Forest fragmentation metrics — edge fraction, core area, connectivity.
1000. Corridors reconnecting fragments, with a measurable effect on fauna (§S).
1001. Assisted migration: planting a species where the climate is going, not
      where it is.
1002. Provenance trials — the same species from different bands performing
      differently.
1003. Forest inventory plots the player establishes and re-measures over years.
1004. Growth-and-yield projection from the inventory.
1005. A forest management plan as an actual document the game tracks against.
1006. Illegal or careless felling having downstream consequences someone else
      experiences (§I item 157).
1007. The colony arguing about the forest, because it is fuel and timber and
      water and soil all at once.
1008. **Rendering: canopy as a height-and-density field** for the distant tiers,
      with individual trees only near. The field is what the sim uses anyway.
1009. Canopy rendered with translucency so light through leaves reads correctly.
1010. Publish the limits: no branch-level architecture, allometry from fitted
      curves, mycorrhizal transfer as a simple sharing rule.

## AA. Trophic structure, fear and the food web (1011–1080)

*ORRERY's `trophicField.js` is the model: producers, grazers, hunters, detritus
as coupled fields, with discrete carcasses and a landscape of fear.*

1011. **Miami-model NPP** from temperature and moisture, as ORRERY does — a
      cited empirical relation replacing any invented biomass curve.
1012. NPP as the single input to the whole food web, so productivity flows
      upward from climate and soil.
1013. Trophic fields per cell: producer, grazer, hunter, detritivore biomass.
1014. **Transfer efficiency around ten per cent** between levels, which is what
      makes predators rare and gives the pyramid its shape for free.
1015. Detritus as its own pool, receiving from every level and feeding the soil.
1016. Decomposer biomass closing the carbon and nutrient loops (§B item 28).
1017. **Kleiber's law** giving population density from body mass — metabolic rate
      scales as mass to the three-quarters, so density scales inversely. Fauna
      numbers derived rather than tuned.
1018. Home range size from the same scaling, so big animals need more drum.
1019. Which means **the drum has a hard upper limit on body size**, because
      33.9 km² cannot support a viable population of anything large. State it,
      and make it a fact the colony knows.
1020. Minimum viable population from that density and the available area.
1021. Species that cannot persist simply do not, and the player can compute why.
1022. Guilds rather than individual species at the T1 tier, aggregated by
      function.
1023. Functional redundancy: several species in a guild means losing one is
      survivable. Losing the last is not.
1024. **Discrete carcasses**: a kill places an object that scavengers find,
      decomposers reduce, and soil eventually receives. Following ORRERY exactly.
1025. Carcass decay stages, visible and smellable to fauna.
1026. Nutrient hotspots where a carcass fell, visible in the soil grid for
      seasons after.
1027. Scavenger guild arriving on a schedule, first the large then the small.
1028. **Landscape of fear**: a predation-pressure field prey read when choosing
      where to graze — the single best behavioural idea available to us.
1029. Which produces **trophic cascades through behaviour, not just numbers**:
      predators keep grazers off the riverbank, so the riverbank vegetates, so
      the bank stops eroding.
1030. Fear decaying with time since the predator passed, so the field breathes.
1031. Refugia — steep ground, dense cover — where fear is low and grazing
      concentrates.
1032. Grazing lawns forming where grazers repeatedly feed, with their own
      distinct vegetation.
1033. Browse lines at a consistent height, visible and diagnostic.
1034. Selective browsing changing species composition, favouring the unpalatable.
1035. Seed dispersal by fauna — gut passage improving germination for some
      species, which ties animals to forest regeneration.
1036. Caching behaviour burying seeds and forgetting some, which is how many
      trees actually spread.
1037. Pollination as a mutualism with a real failure mode (§S item 568).
1038. Pollinator foraging range limiting which fields benefit.
1039. Flowering phenology matching pollinator emergence, and **failing when
      climate shifts one and not the other** — phenological mismatch, a real and
      quietly devastating mechanism.
1040. Parasites and parasitoids as a third trophic path.
1041. Disease dynamics on the population graph, with density-dependent
      transmission.
1042. Host-density thresholds below which a disease burns out.
1043. Immunity and recovery, so epidemics have a shape.
1044. Wildlife disease crossing to livestock and back (§S item 580).
1045. Competition between species for the same resource, with exclusion where it
      is complete.
1046. Niche partitioning letting similar species coexist by using different
      parts of the same resource.
1047. Character displacement over long time, as competitors diverge (§AB).
1048. Keystone species identified by the sim rather than designated — remove
      each in a shadow run and measure the disturbance.
1049. Ecosystem engineers physically modifying terrain (§S item 588).
1050. Foundation species whose structure others depend on.
1051. Mutualisms with real reciprocal benefit, and real collapse if one side goes.
1052. Commensalism and facilitation, which are what make communities more than
      the sum of populations.
1053. **Shannon diversity** computed per region as a health index.
1054. Species-area relationship, which the drum's small closed area makes
      punishing.
1055. Island biogeography applied to fragments — the drum is an island, and its
      fragments are islands within it.
1056. Extinction debt: a fragment too small still holds species that are already
      doomed, and they wink out over decades.
1057. Colonisation and rescue effects along corridors.
1058. Metapopulation dynamics across patches.
1059. Trophic collapse as a legible cascade, not a sudden fail state.
1060. Population graphs and the food web diagram built from live parameters
      (§S items 606–607).
1061. An "if I remove this" simulator, running a shadow model to show the
      consequence before the player acts.
1062. Historical baselines, so the player can see how far the system has drifted.
1063. Regime shift detection — the sim noticing it has crossed into a new stable
      state.
1064. Hysteresis: reversing the cause does not reverse the effect, which is the
      most important and least represented idea in ecology.
1065. Alternative stable states — forest and grassland both stable on the same
      ground, with fire and grazing deciding.
1066. Resilience as a measurable quantity, and a readout of it.
1067. Early-warning signals — rising variance and slowing recovery before a
      shift — which the player can learn to read.
1068. Trophic rewilding as a restoration action with a long payoff.
1069. Reintroduction from the seed and gamete bank, if the genetics survive.
1070. Biological control instead of chemical, with its own risks.
1071. Invasive dynamics along the wind bands and river corridors (§R item 542).
1072. Novel ecosystems that are not what was there and not degraded either.
1073. The colony's own trophic position made explicit — the colonists are a
      guild in this web and consume from it.
1074. Human harvest pressure as a term in every population model.
1075. Sustainable yield computed per species, and visibly exceeded or not.
1076. Bycatch and incidental mortality from farming and building.
1077. Roadkill and infrastructure mortality as fauna meet the colony.
1078. Light pollution from settlements altering fauna behaviour — in a drum, at
      night, your lights are visible across the whole world.
1079. Noise from machinery displacing fauna measurably.
1080. Publish the limits: no individual metabolism, guilds at range, ten per cent
      transfer as a flat assumption, disease as compartment models.

## AB. Evolution, adaptation and the catalogue (1081–1140)

*ORRERY runs open-ended evolution with an expressed genome. In a sealed drum on
a centuries-long voyage, evolution is not a curiosity — it is the plot.*

1081. A genome that **expresses** into a body plan rather than a trait vector
      that interpolates — following ORRERY's `expressBodyPlan`.
1082. Genes with pleiotropy, so one change moves several traits and trade-offs
      are structural rather than authored.
1083. Morph penalty for incoherent body plans, which keeps expression honest.
1084. Mutation on copy, with rate raised by cosmic radiation (§H item 149).
1085. Recombination in sexual species, and clonal lineages in asexual ones.
1086. Horizontal gene transfer between microbes, which is how novelty actually
      spreads at that scale.
1087. Selection as differential survival in the *actual* field conditions, never
      a fitness score.
1088. Local adaptation producing measurably different populations per band —
      which the banded climate (§E) makes inevitable.
1089. Gene flow along corridors reducing that divergence, and its absence
      increasing it.
1090. **Speciation when populations diverge past a threshold**, with a recorded
      split.
1091. A phylogenetic tree the player can browse, growing over the voyage.
1092. Endemism flagged where a lineage exists in one region only.
1093. Adaptive radiation into empty niches after a disturbance.
1094. Convergent evolution producing similar forms in similar bands.
1095. Founder effects when a small group colonises a new region.
1096. Genetic drift in small populations, with real consequences.
1097. Inbreeding depression, and the outcrossing that relieves it.
1098. Genetic load accumulating in the closed population, which is the colony's
      own long-term problem too (`PD_BRIEF.md`).
1099. Purging of deleterious alleles under strong selection.
1100. Heterosis when divergent lines are crossed.
1101. Domestication syndrome emerging from selection for tameness.
1102. Feral reversion when domesticates escape.
1103. Landraces adapted to specific bands, more robust than the improved variety.
1104. Crop wild relatives held in reserve as a genetic resource.
1105. A **seed and gamete bank** as colony infrastructure and insurance.
1106. Cryopreservation exploiting the cold endcaps.
1107. Viability decay in storage, requiring periodic regrowth.
1108. Deliberate mutagenesis as a tool with a radiation cost (§R item 549).
1109. Marker-assisted selection as a late-game accelerator.
1110. Gene editing as an end-game capability with ethical weight in the fiction.
1111. Trait heritability estimated from the actual population, not assumed.
1112. Breeding value prediction improving with records.
1113. A breeding programme UI showing lineage, selection differential and
      realised gain.
1114. Generation time varying by species, so some lineages evolve visibly and
      others do not.
1115. Evolution visible over a playthrough for short-generation species —
      insects, microbes, annual crops.
1116. Pesticide and antibiotic resistance evolving under use, which is the most
      legible evolution mechanic there is.
1117. Pathogen evolution tracking host defences.
1118. Coevolutionary arms races between specific pairs.
1119. **An auto-generated species catalogue**, compiled from data as ORRERY's
      `catalogue.js` is, never hand-edited.
1120. A field guide that fills in as the player observes (§S item 604), keyed to
      the catalogue.
1121. Species entries showing traits, requirements, range, population and
      lineage.
1122. Observation records with date, place and note.
1123. A personal herbarium and specimen collection.
1124. Taxonomy the player can revise as lineages split.
1125. Naming rights: the player names species they discover, and the names
      persist.
1126. A phylogeny visualisation that is beautiful rather than merely correct.
1127. Deep-time playback of the voyage's biological history.
1128. Extinction events recorded with cause and date in the chronicle (§AC).
1129. A red list of species at risk, computed.
1130. Conservation as an actual programme with targets and outcomes.
1131. Ex-situ populations as insurance against in-situ loss.
1132. Rewilding using banked genetics after a collapse.
1133. De-extinction as a very late-game possibility with a real cost.
1134. Biodiversity as a term in the colony's own resilience.
1135. The ship's original manifest as a historical baseline — what they set out
      with, versus what is left.
1136. Species that were lost before the player's generation, known only from the
      manifest and the chronicle.
1137. Which gives the world a **past the player did not cause** and cannot fix,
      and that is where most of its melancholy will come from.
1138. Arrival as the horizon: what reaches the destination is not what left.
1139. The Crèche and the seed bank as the same idea at two scales, which is the
      spine of `PD_BRIEF.md` §4.
1140. Publish the limits: genome as a small expressed vector, speciation by
      threshold, no molecular model, no true fitness landscape.

## AC. The world remembering — traces, chronicle and ruins (1141–1190)

*ORRERY's `whatHappenedHere` is the best single idea in their codebase. A place
that remembers is a place you can care about.*

1141. A **chronicle** of notable events per region: floods, fires, collapses,
      first plantings, extinctions, buildings raised and lost.
1142. Queryable at a point — "what happened here" — as an in-world action.
1143. Events carrying cause, date, magnitude and consequence, reusing item 197's
      causal chains rather than a second system.
1144. Automatic significance filtering, so the chronicle holds the notable and
      not the routine.
1145. Player-authored notes attached to places, becoming part of the record.
1146. The chronicle surviving in saves and being genuinely long-lived.
1147. **Physical traces outlasting the event**: a flood leaves a silt line, a
      fire leaves charcoal in the soil, a landslide leaves a scar.
1148. Charcoal horizons in the soil column readable by digging (§Q item 444).
1149. Silt layers in lake beds as an annual record — varves, effectively.
1150. Tree rings as a second independent archive (§Z item 954), which the player
      can cross-check against the first.
1151. Terrace treads recording old river levels (§O item 291).
1152. Abandoned channels visible as vegetation lines long after the water left.
1153. Old field boundaries persisting as soil differences and hedgerows.
1154. Ridge and furrow from long-abandoned ploughing, still visible in raking
      light.
1155. Spoil heaps and quarry faces from the builders (§N items 232–234).
1156. Collapsed workings and subsidence hollows above them (§N item 235).
1157. Ruins of earlier settlements, decaying on a real clock.
1158. Structures the player abandons decaying identically — no special case for
      the player's own things.
1159. Decay stages visible: intact, weathered, collapsed, overgrown, mounded.
1160. Vegetation succession reclaiming ruins (§G item 134).
1161. Nettles and nutrient-loving species marking where people lived, which is
      how archaeologists actually find settlements.
1162. Middens as nutrient-rich, artefact-rich soil.
1163. Artefacts recoverable by digging, with a date and a story.
1164. The ship's own archaeology: what the builders left, what previous
      generations built and lost.
1165. Graves and memorials as permanent, respected features.
1166. Named places accumulating names from events rather than from a generator.
1167. Place names the colony uses in dialogue, so the world is talked about.
1168. Paths worn by use (§Q item 487) and fading when unused.
1169. Desire lines emerging from actual movement, not authored.
1170. Wear on stone thresholds and steps, over years.
1171. Patina and weathering on everything the player builds.
1172. Repair visible as newer material against older.
1173. Graffiti and marks left by previous generations.
1174. A settlement's age readable from its fabric.
1175. **The Archive** (`PD_BRIEF.md` §8.1) as the colony's own memory system,
      which the chronicle feeds — the same idea in fiction and in code.
1176. Recorded moments the player can revisit, tied to place.
1177. Photographs or their analogue, taken and kept.
1178. A generational timeline showing the whole voyage.
1179. Previous colonists' journals found in ruins.
1180. Their farms visible as soil signatures under your own.
1181. Their mistakes visible in the landscape — the eroded slope, the salted
      field, the silted reservoir.
1182. **Your mistakes becoming that** for whoever comes next, which is the point.
1183. Long-term soil trend graphs spanning generations.
1184. Habitat-scale trend history for every budget in §J.
1185. An honest end-of-voyage report against the original manifest.
1186. Comparison views: this region now, versus fifty days ago, versus a century.
1187. Time-lapse of any region from stored state (§R item 535).
1188. A map that accumulates annotations, routes and knowledge over play.
1189. Fog of knowledge rather than fog of war — the map shows what has been
      *surveyed*, and survey is a real activity.
1190. Publish the limits: chronicle as filtered events, traces as authored
      consequences of simulated causes, not a full historical simulation.

## AD. Sensory magic — what makes a place breathe (1191–1270)

*Everything above is machinery. This section is why anyone would stand still and
look at it.*

1191. **Dawn is an event.** The strip ramps, mist lifts off the river, birds
      start, dew evaporates, temperature climbs. Five systems already exist;
      choreograph them.
1192. Dusk as its own event, with the far side of the drum lighting up as
      settlements switch on — the most spectacular thing this geometry offers.
1193. Mist forming in valley bottoms overnight and burning off in bands.
1194. River fog following the water exactly, because it is generated from the
      water temperature and air temperature difference.
1195. Steam rising off wet ground when the strip comes up.
1196. Dew on grass rendering as a brief sparkle at grazing angles.
1197. Frost on the shadowed side of things, melting first where light lands.
1198. The first frost of a cold band as a noticed, dated event.
1199. Seasonal firsts logged: first flower, first fruit, first ice. Phenology as
      a felt thing, not a number.
1200. Leaves turning by species over days, not all at once.
1201. Leaf fall on a windy day as a visible, audible event.
1202. Bare-branch light in the cold season reaching the forest floor, waking the
      ground flora.
1203. Snow arriving quietly and changing every sound in the world.
1204. Snowmelt as an audible increase in river noise days before the flood.
1205. **Rain you can see coming** across the drum — a curtain under a band,
      travelling, arriving.
1206. The sound arriving before the rain does.
1207. Petrichor cue as the humidity spikes (§P item 400).
1208. The wind picking up before a storm, visible in the canopy first.
1209. Animals reacting before the weather does, which is how people actually
      forecast.
1210. Birds going quiet before something happens.
1211. Insect sound rising with temperature, so the air is audibly warmer.
1212. Evening insect swarms over water, catching the light.
1213. Fireflies or their analogue on warm still nights.
1214. Bioluminescence in caves and in still water (§S).
1215. Fish rising, rings spreading on a lake at dusk.
1216. Birds mobbing a predator, visible from a long way off and meaning something.
1217. A herd running, which tells you something happened before you know what.
1218. Tracks in mud and snow that you can read and follow (§S item 585).
1219. Sound occlusion by terrain, so a river is audible round a corner and not
      through a hill.
1220. Echo off the far side of the drum — the sound of an enclosed world, which
      nothing else can offer (§P item 382).
1221. Reverb varying by biome: dense forest deadens, open water carries, a cave
      rings.
1222. The wind in different vegetation making different sound — grass hiss,
      broadleaf rustle, needle roar.
1223. Rain on different surfaces: leaves, water, stone, roof, mud.
1224. Footstep sound and feel by material and wetness, from the fields we have.
1225. Your own breathing at altitude and under load.
1226. Tool sound by material struck, pitched by the material's hardness — the
      audio system already generates procedurally, so this is nearly free.
1227. Distance-appropriate audio content: a settlement heard as murmur, then
      voices, then words.
1228. A quiet mode where the machinery stops and you notice how loud it was.
1229. **Light through the canopy from the axis** — near-vertical, converging,
      unlike any sunbeam (§U item 473).
1230. Dappled light moving with the canopy sway.
1231. Light shafts through cave openings, dust turning in them.
1232. The strip's reflection on every water surface, running the length of the
      drum toward you.
1233. Reflections of the far side in still water — the world upside down in a
      lake (§U item 700).
1234. Wind ruffling a reflection away and letting it return.
1235. Colour of the light changing through the day cycle, and the ground
      answering.
1236. **Bounce light from the far side tinted by what grows there** (§U item
      691) — the single most magical possible detail, and physically correct.
1237. Weather on the far side visible as a distant band of rain, overhead.
1238. Watching a storm you are not in, from a hillside.
1239. Watching your own fields from across the drum, tiny and green.
1240. The drum's curve as a constant, quiet strangeness that never quite stops
      being odd.
1241. Objects thrown curving, which never stops being funny (already built).
1242. Rain falling at an angle that is not the wind's angle.
1243. Smoke rising and then bending, because Coriolis acts on it too.
1244. Dust devils that spiral the way the drum's rotation dictates.
1245. Seeds and spores drifting along the bands (§H item 148), visible in low
      light.
1246. Pollen haze in flowering season, thick enough to see.
1247. Insect clouds over a warm field.
1248. Flocking birds wrapping around the drum, which is a shot no other game has.
1249. Animal calls at distance, located and identifiable.
1250. Grazing herds visible on the far side as slow-moving specks.
1251. Small mammals startling from cover as you walk through it.
1252. Butterflies and pollinators on flowering plants, density from the actual
      pollinator population.
1253. Bees returning to a hive you built, in a stream you can watch.
1254. Water striders and surface tension on still water.
1255. Ripples spreading from a dropped stone, with the right speed.
1256. A leaf landing on water and drifting downstream at the real flow velocity.
1257. Floating debris tracking the current, which makes flow legible.
1258. Foam collecting in eddies and slowly turning.
1259. Sediment plumes where a tributary joins in flood (§O item 287).
1260. A river running clear after rain has passed, over days.
1261. Puddles shrinking through a morning.
1262. Mud drying and cracking into polygons.
1263. Grass laying over where something walked, and standing up again.
1264. Crops moving in a wave across a field, which is wind made visible.
1265. Fruit heavy enough to bend a branch.
1266. Growth you can see if you sit still — subtle, but present.
1267. A place you return to being visibly different, every time.
1268. **Seasonal light angle** changing what the place looks like, entirely, four
      times a cycle.
1269. A view you deliberately built a bench to look at, and the game noticing.
1270. Publish the limits: mood is authored choreography over simulated fields —
      the fields are honest, the choreography is art direction.

## AE. The ground beneath — microbial, fungal and small life (1271–1310)

*The most important organisms in any landscape are the ones you cannot see, and
`soil.rs` already has a slot for them.*

1271. Microbial biomass as a live field driving mineralisation rate (§B item 28).
1272. Microbial community composition — bacteria, fungi, actinomycetes — shifting
      with pH and moisture.
1273. Fungal-to-bacterial ratio as a diagnostic of soil condition: forests
      fungal, grassland bacterial.
1274. Nitrogen fixers as a specific population, boosted by legumes (§B item 29).
1275. Nitrifiers and denitrifiers, with denitrification losing nitrogen to the
      atmosphere — a real loss term in the closed ledger.
1276. Mycorrhizal colonisation rate per plant, improving nutrient uptake.
1277. The mycorrhizal network as a graph (§Z item 962).
1278. Soil respiration as a measurable carbon flux, rising with temperature.
1279. Which makes warming a **positive feedback** on carbon release, correctly
      signed and quietly alarming.
1280. Priming: fresh organic input accelerating decomposition of old carbon.
1281. Aggregate stability from fungal hyphae and microbial glues, resisting
      erosion.
1282. Soil structure as a state that takes years to build and one ploughing to
      destroy.
1283. Earthworms and their analogues improving structure and drainage.
1284. Worm casts on the surface as a visible sign of good soil.
1285. Soil arthropods shredding litter before microbes finish it.
1286. Springtails and mites as the base of the soil food web.
1287. Nematodes, both beneficial and parasitic.
1288. A soil food web diagram alongside the surface one.
1289. Suppressive soils where the microbial community resists pathogens.
1290. Soil-borne disease building under monoculture, which is the mechanism
      behind rotation.
1291. Inoculation as a deliberate action — introducing a microbial community.
1292. Compost teas and starters as low-tech inoculants.
1293. Fungal fruiting as the visible sign of an invisible network (§Z item 964).
1294. Mushroom foraging with real species that matter.
1295. Cultivated fungi on substrate (§Y item 939).
1296. Decomposer succession on deadwood, species by species, over years.
1297. Lichens as slow colonisers of bare rock, and the first step of soil
      formation (§N item 247).
1298. Lichen cover as a dating tool for exposed surfaces.
1299. Biological soil crusts on bare ground, fragile and slow to recover.
1300. Crust destruction by trampling, with a decade-long recovery.
1301. Rock weathering accelerated by biological activity.
1302. **Soil formation from bare regolith** as a multi-generation project the
      player can deliberately undertake — terraforming, at the scale that
      actually matters.
1303. Which makes `material.rs`'s regolith a *raw material for making soil*,
      linking §X directly to §B.
1304. Soil depth increasing measurably where the player has managed it well.
1305. A soil profile view showing horizons developing over time.
1306. Horizon differentiation — organic, leached, accumulated — emerging.
1307. Soil colour changing as organic matter accumulates, visible in the world.
1308. A microscope or assay station for reading the invisible.
1309. The soil overlay as the place all of this becomes legible (§B item 34).
1310. Publish the limits: microbial pools as lumped biomass, no species-level
      microbiology, mineralisation as a first-order rate.

## AF. Emergence, colony and story (1311–1360)

*Where the simulation stops being a landscape and starts being a place people
live in.*

1311. Colonists as agents with needs the biosphere actually supplies: calories,
      protein, water, warmth, air.
1312. Nutritional composition from the crops' actual allocation (§R item 527), so
      a diet of one crop fails in a specific way.
1313. Labour as the scarce resource it really is (§Y item 943).
1314. Skills improving with use, so a colonist who farms becomes a farmer.
1315. Knowledge as a colony asset that can be lost with a person.
1316. Apprenticeship transferring it deliberately.
1317. Written records surviving people, which is what the Archive is for.
1318. Population dynamics driven by the Crèche and by mortality.
1319. Dependency ratio: how many workers support how many non-workers.
1320. Carrying capacity computed from arable area and yield — a number the colony
      can watch approach.
1321. Which makes **the Crèche a resource-allocation decision**, exactly as
      `PD_BRIEF.md` §4 argues, with the soil grid supplying the constraint.
1322. Settlement growth following food, water and materials rather than a build
      menu.
1323. Settlements sited where the terrain, water and soil actually favour them.
1324. Regional specialisation emerging from what each band can produce.
1325. Trade between settlements moving material along real routes with real cost.
1326. Surplus and shortage as the driver of that trade.
1327. Price or its analogue emerging from scarcity, not authored.
1328. Infrastructure — paths, bridges, channels — as shared investment.
1329. Commons problems: shared water, shared forest, shared soil, no outside.
1330. Governance as a mechanic, since the drum forces collective decisions.
1331. Votes on daylight length, power allocation and Crèche slots (§D item 72).
1332. Factions forming around genuine material interests — upstream against
      downstream, forest against forge.
1333. **The upstream–downstream relationship as the colony's central politics**
      (§I item 170), because it is physically true.
1334. Disputes the player can arbitrate, with real consequences either way.
1335. Reputation with regions and factions, earned through material acts.
1336. Colonists commenting on what they can see: a failing crop, a new channel,
      a felled wood.
1337. NPC memory of the player's specific actions, per `PD_BRIEF.md` §6's memory
      ledger.
1338. Relationships deepening through shared work in shared places.
1339. A character's opinion of a *place*, not only of you.
1340. Grief and attachment when a place is lost — the flooded field, the burnt
      wood.
1341. Festivals tied to the phenological calendar the sim produces.
1342. Harvest as a communal event whose scale depends on the actual harvest.
1343. Rituals marking the seasonal firsts (§AD item 1199).
1344. Naming ceremonies for places and for children.
1345. The chronicle read aloud, so history is spoken as well as stored.
1346. Emergent stories from system collisions — the drought that forced the
      felling that silted the reservoir that failed the harvest.
1347. The game recognising such a chain and offering it as a narrative beat.
1348. Long-horizon consequences arriving a generation later, when the cause is
      only in the chronicle.
1349. Playing a descendant, with your predecessor's landscape as your
      inheritance.
1350. Their choices visible in the soil, the drainage and the forest.
1351. An ending that is a **state**, not a victory: what the drum looks like when
      it arrives.
1352. The arrival report against the original manifest (§AB item 1135).
1353. Multiple honest endings — a thriving drum, a bare one, a wild one with few
      people, a crowded one with poor soil.
1354. No ending that is simply "you won", because the system does not work that
      way.
1355. A last save the player can walk around in afterwards.
1356. New game plus seeded with the previous drum's genetics and chronicle.
1357. A shared habitat other players visit (`PD_BRIEF.md` §7), where what they
      see is your landscape and your record.
1358. Their drum's state visible for comparison, which is the whole social hook.
1359. The Committee mode competing over the same finite Crèche slots and the
      same finite soil.
1360. Publish the limits: colonists as need-driven agents, no economics engine,
      politics as authored structures over simulated pressures.

## AG. Architecture to carry all of this (1361–1400)

*Fourteen hundred items is a way to build an unmaintainable pile. These are the
constraints that stop it.*

1361. **One tick order, written down**: weather → hydrology → soil → plants →
      fauna → economy → chronicle. `biosphere.rs` already owns this; make the
      order explicit and tested.
1362. No system reads another's state mid-tick; everything reads last tick's
      values. Double-buffer the fields that matter.
1363. A dependency graph of systems, generated and checked, so cycles are caught.
1364. Every field registered in the curated schema (§W item 771) with owner, unit
      and save status.
1365. A field census failing CI when an unregistered field appears.
1366. Units in the type system where possible, so metres and cells cannot be
      confused.
1367. One coordinate convention documented once: (theta, z, r), theta wraps, r
      measured inward from the hull.
1368. Conversion helpers in one place, never inlined per call site.
1369. The presentation layer (§W) as the only path from sim state to renderer.
1370. Renderers forbidden from reaching into simulation arrays, enforced by the
      architecture ratchet.
1371. Every derived quantity computed once per frame and cached, not
      recalculated per consumer.
1372. Tier promotion and demotion as one shared mechanism, not per system
      (`SIM_ARCH_BRIEF.md` §1).
1373. A parity test per system between its T0 and T1 forms (§W item 775).
1374. Determinism as a property tested continuously, not assumed (§W item 773).
1375. Seeded RNG streams per system per region, so adding a system cannot change
      another's sequence.
1376. Fixed-point or strictly-ordered float accumulation where sums must be
      reproducible.
1377. Save format versioned with migrations tested against fixture saves
      (`REQUIREMENTS.md` E5) — `persist.rs` exists, keep it honest.
1378. Deltas only: never serialise a derived field.
1379. Save size budgeted and measured as content grows.
1380. A time budget per system, enforced, with a profiler overlay showing the
      actual split (§V item 752).
1381. Systems that exceed budget degrade gracefully rather than spiking a frame.
1382. All heavy work off the main thread, with results applied at a defined point.
1383. Incremental everything: dirty regions as a shared primitive (§V item 759).
1384. A headless runner for long-horizon experiments (§W item 787).
1385. Batch runs across seeds reporting distributions (§W item 788).
1386. Golden tests for each system, plus one for the coupled whole.
1387. A calibration table with provenance tags for every empirical constant
      (§W item 776) — Miami model, Kleiber exponent, transfer efficiency, angle
      of repose, all cited.
1388. Constants that are invented tagged as invented, visibly.
1389. `MODEL_LIMITS.md` updated **before** any system claims more precision.
1390. A model-limits assertion: if a system's output range exceeds what the doc
      claims, the test fails.
1391. Data-driven everything — materials, recipes, species, paint rules, biome
      thresholds — hot-reloadable and schema-validated.
1392. A single authoring tool a designer can use for all of it.
1393. `NEXT.md` as the only backlog, capped at ten (§W item 799). These 1,400
      items are a **register, not a queue**.
1394. Each item traceable to the requirement it serves in `REQUIREMENTS.md`.
1395. Requirements traceable to a test.
1396. A quality register in this same numbered form for defects.
1397. Playtest instrumentation on the first ninety seconds (§W item 794).
1398. An accessibility audit as a gate, not an intention (§T items 629–631).
1399. Regular holistic reviews that **measure** rather than assume — this
      document opened by running the selftest and finding the coupling now works,
      which is the only reason its first page is worth reading.
1400. And the rule that governs all fourteen hundred, unchanged: **honesty of
      mechanism beats cosmetic spectacle; legibility beats completeness; delight
      in the first ninety seconds beats feature count; stated limits beat implied
      precision.**

---

## What to build next, given what now exists

The measured state changes the answer. Flow, soil, weather, plants, lakes and
biomes are in. The gap is that **none of it is in your hands**.

| Wave | Items | Why this, now |
|---|---|---|
| **1** | 801–806, 813, 820–822, 831–832 | **Digging yields material.** Mass, volume, inventory, encumbrance, and recipes as mass-balanced data. Nothing else in this document matters until the world can be picked up. |
| **2** | 839–843, 851–855, 869–871 | The first closed loop: forest → charcoal → smelt → metal, paid for in oxygen; and sand → glass → greenhouse → yield. Mining sand makes food. |
| **3** | 1011–1017, 1024, 1028–1029 | Miami NPP, Kleiber densities, carcasses, and the landscape of fear. The food web on top of the productivity the sim already computes. |
| **4** | 1141–1148, 1175 | The chronicle and physical traces. The world starts remembering, and the Archive gets something to hold. |
| **5** | 1191–1195, 1205–1206, 1236 | Dawn, dusk, rain you can see coming, and the far-side bounce light. Choreograph what already exists. |
| **6** | 1361–1370, 1387–1390 | Tick order, presentation boundary and the calibration table — before the pile gets bigger. |

**The loop I would build first, end to end, in one week:** dig clay, fire it in
a kiln fuelled by charcoal from a wood you felled, and use the pipe you made to
irrigate a field whose soil moisture visibly rises. Five systems that already
exist, joined by one that does not.

**And the item to build for magic** is 1236. The ambient light in this world is
bounce off the far wall, which is sunlit farmland eighteen hundred metres
overhead. Tint it by what actually grows up there, and when a forest burns on
the far side, the light on your hands changes colour. It is physically correct,
it is nearly free, and no open-world game can do it — because none of them are
closed.
