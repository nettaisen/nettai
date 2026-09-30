--!strict
-- MegaMan's buster: weapon routine 0 (`sub_8011A26`), which sets up
-- action 0x11 with the buster's damage. The NaviCust's buster bugs may make
-- a shot a blank or a charged shot instead (one RNG draw every shot), and
-- its buster-shot program a spread shot (another draw).

local blank_shot = require("../02-blank-shot/blank_shot")
local charged_shot = require("../01-charged-shot/charged_shot")

local buster = {}

local ACTION = 0x11

-- The buster-shot programs from this one on spread on 1 draw in 8, the
-- others on 1 in 2.
local RARE_SPREAD = 0x1E

-- `sub_8013D5E`: what this shot is. Of 16 slots, the first `buster_blanks`
-- are blanks and the next `buster_charged` charged shots; one draw picks a
-- slot.
local function pick(navi: Object): "buster" | "blank" | "charged"
    local stats = battle.navi(navi.alliance)
    local blanks, charged = stats.buster_blanks, stats.buster_charged
    if blanks + charged > 16 then
        error(
            string.format(
                "%d blank and %d charged slots overflow sub_8013D5E's 16-slot table (over its stack frame)",
                blanks,
                charged
            )
        )
    end
    local i = bit32.band(battle.rng(), 0xF)
    if i < blanks then
        return "blank"
    elseif i < blanks + charged then
        return "charged"
    end
    return "buster"
end

-- `sub_8011A26`.
function buster.setup(navi: Object): number
    local shot = pick(navi)
    if shot == "blank" then
        return blank_shot.setup(navi)
    elseif shot == "charged" then
        return charged_shot.setup(navi)
    end
    navi.attack_damage = navi:buster_damage()
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    -- The buster-shot program (the projectile's kind), on a lucky draw.
    local spread = battle.navi(navi.alliance).buster_shot
    if spread ~= 0 then
        local mask = if spread >= RARE_SPREAD then 7 else 1
        if bit32.band(battle.rng_positive(), mask) ~= 0 then
            spread = 0
        end
    end
    navi:set_attack_param(1, spread)
    navi.variant = 0
    return ACTION
end

return buster
