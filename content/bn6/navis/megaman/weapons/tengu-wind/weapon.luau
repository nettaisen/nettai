--!strict
-- TenguCross's B+Back special (`megaman/tengu-wind`): the original's
-- weapon routine 0x10 (`sub_8011CA8`), which names the instant chips'
-- action with an effect no chip has (`sub_80CD4AC`, the original's instant
-- effect 0x14) and a 40-tick cooldown before the next B+Back. The effect
-- blows a gust (objects/gust) down each row from the far column toward the
-- navi's own side; the navi then waits 8 ticks (the instant chips' action's
-- wait for this effect).

local gust = require("../../../../objects/gust/gust")

local tengu_wind = {}

local COOLDOWN = 0x28

local SOUND = asset.sound("dustbrk")

-- The gusts pull: the weak push, turned toward the navi.
local PULL: GustHow = { pulls = true }

-- `sub_8011CA8`: the cooldown; its effect is the weapon's `instant`, which
-- the instant chips' action runs.
local function setup(navi: Object)
    navi.attack_lockout = COOLDOWN
    return nil
end

-- `sub_80CD4AC`: a gust in each row, from column 6 when the navi faces
-- right (1 when it faces left). The gusts' side is the navi's shifted by
-- its flip (the game's `lsl`), their flip 0; no damage, nothing tracks
-- them.
local function blow(user: Object, _spec: InstantChipSpec)
    local facing = bit32.bxor(user.alliance, user.flip)
    local x = 5 * bit32.bxor(facing, 1) + 1
    local flip = user.flip
    local word = if flip < 32 then bit32.lshift(user.alliance, flip) else 0
    local side, side_flip = bit32.band(word, 0xFF), bit32.band(bit32.rshift(word, 8), 0xFF)
    for y = 1, 3 do
        gust.spawn(x, y, side, side_flip, PULL, 0, nil, 0)
    end
    battle.play_sound(SOUND)
end

tengu_wind.weapon = define.weapon {
    id = "megaman/tengu-wind",
    name = "Tengu wind",
    -- The charge table's row 0x10, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 60, 60, 60 },
    setup = setup,
    instant = blow,
}

return tengu_wind
