--!strict
-- What the ruleset needs from the test pack by role
-- (docs/design/content-model-v2.md §7.4): AntiDmg's counter and the
-- charged shot a navi's request starts are the pack's own tickers; the
-- rest is the test content's (BN6's modules), which this pack's content
-- comes with.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local beast_claw = require("../../navis/megaman/weapons/beast-claw/weapon")
local beast_rush = require("../../rules/beast/rush")
local dust_beast_scatter = require("../../navis/megaman/forms/dustcross-beast/scatter")
local encased_bubble = require("../../objects/encased-bubble/bubble")
local falling_rock = require("../../objects/falling-rock/falling_rock")
local ground_drill = require("../../navis/megaman/forms/groundcross/drill")
local slash_beast_lunge = require("../../navis/megaman/forms/slashcross-beast/lunge")
local slash_charge = require("../../navis/megaman/forms/slashcross/charge")
local stun_strike = require("../../lib/swords/stun_strike")
local test_chips = require("../../chips/test/chips")
local turn = require("../../navis/megaman/turn")
local lockon = require("../../rules/lockon")
local ruleset = require("../../rules/ruleset")
local statuses = require("../../rules/status")
local ticker = require("../lib/ticker")

return define.roles {
    id = "test:roles",
    actions = {
        anti_damage_counter = ticker.action { id = "test:test/anti-damage-counter", length = 4 },
        forced_charged_shot = ticker.action { id = "test:test/forced-charged-shot", length = 3 },
        stun_strike = stun_strike.action,
        cross_protect = ground_drill.action,
        turn = turn.action,
        cross_death = slash_beast_lunge.action,
        charged_sword = slash_charge.action,
        beast_claw = beast_claw.action,
        beast_rush = beast_rush.action,
        dust_beast_scatter = dust_beast_scatter.action,
    },
    kinds = {
        absorbed_obstacle = absorbed_obstacle.kind,
        falling_rock = falling_rock.kind,
    },
    chips = {
        -- What a zeroed chip field reads: the test content's blank chip.
        zeroed = test_chips.blank,
    },
    -- The engine's test content's statuses, lock-on modes, effects, sparks,
    -- region and collision types, which this pack's content comes with.
    statuses = {
        damage_word_paralysis = statuses.paralyze_90,
        counter_paralysis = statuses.paralyze_150,
        ice_freeze = statuses.freeze_150,
        hit_bug_blind = statuses.blind_1200,
        hit_bug_confuse = statuses.confuse_960,
    },
    lockon = {
        beast_claw = lockon.beast_claw,
    },
    effects = ruleset.effects,
    sparks = ruleset.sparks,
    regions = ruleset.regions,
    collision = ruleset.collision,
    sounds = ruleset.sounds,
    music = ruleset.music,
    sprites = ruleset.sprites,
    banners = ruleset.banners,
    hooks = {
        encased = encased_bubble.replace,
    },
}
