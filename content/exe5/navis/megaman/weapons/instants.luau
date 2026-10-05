--!strict
-- The patch cards' charged shots that are instant chip effects (the instant chips' action, EXE5's 0x1A, 0x080EC6F6:
-- the effect, then back to idle at once): weapon routines that name the action with an effect of theirs.
--
-- - B↓Invis (`megaman/invisible`, routine 0x47, 0x0800FC3A): effect 2 (0x0800E184, EXE6's `sub_8010474`: the same
--   code but for the timer's place in EXE5's collision record): the navi turns invisible, flashing, for 120 ticks
--   (the attack's parameters' low halfword), with its sound. (The routine writes only the parameters' first byte:
--   the second, which the timer's high byte reads, is what the last attack left. The engine keeps no attack
--   parameters, so it reads 0 here.)
-- - B↓Vacuum (`megaman/vacuum`, routine 0x4C, 0x0800FC82): effect 25 (0x080CEBC8, EXE6's `sub_80CD4AC`, TenguCross's
--   wind: the same code): a pulling gust (navis/megaman/gust) down each row from the far column, with its sound.

local common_gust = require("@exelib/gust")
local gust = require("../gust")

local instants = {}

local INVISIBLE_TICKS = 120
local INVISIBLE_SOUND = asset.sound("invisible")
local VACUUM_SOUND = asset.sound("dustbrk")

-- The routines name no action of their own: the instant chips' action runs their effect.
local function names_effect(_navi: Object)
    return nil
end

-- 0x0800E184: invisible and flashing for the ticks, with the sound.
local function vanish(user: Object, _spec: InstantChipSpec)
    user:set_status_timer("flash", INVISIBLE_TICKS)
    user:set_status("invisible", true)
    battle.play_sound(INVISIBLE_SOUND)
end

instants.invisible = define.weapon {
    id = "megaman/invisible",
    -- The charge table's row 0x47, by Charge 0 to 5: past 4 it reads on into row 0x48's first entry.
    charge_ticks = { 160, 155, 150, 145, 140, 80 },
    setup = names_effect,
    instant = vanish,
}

-- 0x080CEBC8: three pulling gusts, the sound.
local function vacuum(user: Object, _spec: InstantChipSpec)
    common_gust.blow_rows(gust.kind, user, VACUUM_SOUND)
end

instants.vacuum = define.weapon {
    id = "megaman/vacuum",
    -- The charge table's row 0x4C, by Charge 0 to 5: past 4 it reads on into row 0x4D's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 90 },
    setup = names_effect,
    instant = vacuum,
}

return instants
