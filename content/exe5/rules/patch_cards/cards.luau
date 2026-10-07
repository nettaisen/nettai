--!strict
-- EXE5's patch cards (its Modification Cards; docs/design/exe5-map.md §15.14): a card is the engine's
-- (`define.patch_card`, content/exe5/patch_cards/), its effects the values the shared constructors make
-- (@exelib/patch_cards/effects, EXE6's too); rules/patch_cards (./init.luau) applies a player's installed
-- cards to the navi's stats as the round is set up. EXE5's own are its tables: the kinds' order (its effect numbers,
-- 0x00 to 0x89, the handlers at 0x08137BC8) and the trails' bytes (0x08138802).

local effects = require("@exelib/patch_cards/effects")

local patch_cards = {}

export type Effect = PatchCardEffect

-- The kinds in the order they apply (the original's effect numbers: 0x00 HP+ ... 0x89 BugStop). Hub Style, which
-- no effect number names, is card 111's (0x08137A58).
patch_cards.ORDER = {
    "hp_add",
    "hp_percent_add",
    "hp_sub",
    "hp_percent_sub",
    "body",
    "attack_add",
    "attack_sub",
    "attack_times",
    "rapid_add",
    "rapid_sub",
    "charge_add",
    "charge_sub",
    "custom_add",
    "custom_sub",
    "mega_add",
    "mega_sub",
    "giga_add",
    "giga_sub",
    "soul_time_add",
    "soul_time_sub",
    "super_armor",
    "status_guard",
    "float_shoes",
    "air_shoes",
    "undershirt",
    "b_button",
    "buster_shot",
    "charged_shot",
    "charge_shot_kind",
    "first_barrier",
    "back_special",
    "chip_recovery",
    "gauge",
    "rush",
    "beat",
    "tango",
    "sneak_run",
    "encounter_element",
    "millions",
    "auto_heal",
    "mega_virus",
    "step_bug",
    "panel_trail",
    "buster_blanks",
    "hit_status",
    "custom_damage",
    "emotion_bug",
    "hp_drain",
    "custom_hp_drain",
    "status_bug",
    "bug_stopper",
    "hub_style",
} :: { string }
patch_cards.RANK = {} :: { [string]: number }
for i, kind in patch_cards.ORDER do
    patch_cards.RANK[kind] = i
end

-- The kinds whose choice is part of what the effect is (the original has an effect number per choice), and the
-- choice's field.
patch_cards.CHOICE = {
    body = "element",
    b_button = "weapon",
    buster_shot = "variant",
    charged_shot = "weapon",
    charge_shot_kind = "variant",
    first_barrier = "barrier",
    back_special = "weapon",
    gauge = "gauge",
    panel_trail = "trail",
    hit_status = "status",
    encounter_element = "what",
} :: { [string]: string }

-- The element bytes of the bodies (NaviStats +0x10).
patch_cards.BODY = { null = 0, fire = 1, aqua = 2, elec = 3, wood = 4 } :: { [string]: number }
-- What a step leaves behind, by the panel the effect names (the trail byte, NaviStats +0x12: 0x08138802's rows,
-- EXE5's panel types: water its sea, magnet its magnet panel, holy its 9).
patch_cards.TRAILS = {
    crack = 3,
    hole = 1,
    water = 0x0A,
    ice = 7,
    grass = 6,
    magnet = 5,
    holy = 9,
    poison = 4,
    normal = 2,
    none = 0,
} :: { [string]: number }
-- The status a hit gives (NaviStats +0x16, 0x0813880D).
patch_cards.HIT_STATUS = { blind = 1, confuse = 2, hp_bug = 3, none = 0 } :: { [string]: number }
-- The random encounters' element (NaviStats +0x27, 0x081387FC), by program.
patch_cards.ENCOUNTER = { oil_body = 2, fish = 4, battery = 8, jungle = 0x10, search = 1 } :: { [string]: number }

-- ---- The effects (@exelib/patch_cards/effects, EXE6's too) ----------------

patch_cards.bug = effects.bug
patch_cards.hp = effects.hp
patch_cards.hp_percent = effects.hp_percent
patch_cards.body = effects.body
patch_cards.attack = effects.attack
patch_cards.rapid = effects.rapid
patch_cards.charge = effects.charge
patch_cards.custom = effects.custom
patch_cards.mega_folder = effects.mega_folder
patch_cards.giga_folder = effects.giga_folder
patch_cards.soul_time = effects.soul_time
patch_cards.attack_times = effects.attack_times
patch_cards.super_armor = effects.super_armor
patch_cards.status_guard = effects.status_guard
patch_cards.float_shoes = effects.float_shoes
patch_cards.air_shoes = effects.air_shoes
patch_cards.undershirt = effects.undershirt
patch_cards.rush = effects.rush
patch_cards.beat = effects.beat
patch_cards.tango = effects.tango
patch_cards.sneak_run = effects.sneak_run
patch_cards.encounter_element = effects.encounter_element
patch_cards.millions = effects.millions
patch_cards.auto_heal = effects.auto_heal
patch_cards.mega_virus = effects.mega_virus
patch_cards.b_button = effects.b_button
patch_cards.buster_shot = effects.buster_shot
patch_cards.charged_shot = effects.charged_shot
patch_cards.charge_shot_kind = effects.charge_shot_kind
patch_cards.first_barrier = effects.first_barrier
patch_cards.back_special = effects.back_special
patch_cards.chip_recovery = effects.chip_recovery
patch_cards.gauge = effects.gauge
patch_cards.step_bug = effects.step_bug
patch_cards.panel_trail = effects.panel_trail
patch_cards.buster_blanks = effects.buster_blanks
patch_cards.hit_status = effects.hit_status
patch_cards.custom_damage = effects.custom_damage
patch_cards.emotion_bug = effects.emotion_bug
patch_cards.hp_drain = effects.hp_drain
patch_cards.custom_hp_drain = effects.custom_hp_drain
patch_cards.status_bug = effects.status_bug
patch_cards.bug_stopper = effects.bug_stopper
patch_cards.hub_style = effects.hub_style

-- What the application reads of EXE5's (@exelib/patch_cards/apply).
patch_cards.GAME = {
    rank = patch_cards.RANK,
    choice = patch_cards.CHOICE,
    bodies = patch_cards.BODY,
    hit_status = patch_cards.HIT_STATUS,
} :: PatchCardsGame

return patch_cards
