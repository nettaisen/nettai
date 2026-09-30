--!strict
-- MegaMan's charged shot: weapon routine 1 (`sub_8011A7E`), which sets up
-- action 0x16 with (Attack + 1) * 10 damage (10 when worn out). The
-- NaviCust's charged-shot program changes the shot's kind on a lucky draw.

local charged_shot = {}

local ACTION = 0x16

-- The plain charged shot's kind.
local PLAIN = 6

-- `sub_8011A7E`.
function charged_shot.setup(navi: Object): number
    local side = navi.alliance
    local stats = battle.navi(side)
    local base = stats.attack + 1
    if battle.emotion(side) == "worn_out" then
        base = 1
    end
    navi.attack_damage = base * 10
    -- The kind: the program's on 1 draw in 8 (kinds 9 and 0x23) or 1 in 2.
    local kind = stats.charge_shot_kind
    if kind == 0 then
        kind = PLAIN
    elseif kind ~= PLAIN then
        local mask = if kind == 9 or kind == 0x23 then 7 else 1
        if bit32.band(battle.rng_positive(), mask) ~= 0 then
            kind = PLAIN
        end
    end
    navi:set_attack_param(1, kind)
    for i = 2, 4 do
        navi:set_attack_param(i, 0)
    end
    navi.attack_element = 0
    navi.variant = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return ACTION
end

return charged_shot
