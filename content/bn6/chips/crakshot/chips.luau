--!strict
-- CrakShot, DublShot and TrplShot (the original's action 0x22,
-- `sub_80EC960`): the navi digs into the panels ahead and flings them. On
-- the third tick one crack shot (shot.luau) leaves from each panel of the
-- chip's region, taken from the panel ahead: that panel (CrakShot), it
-- and the one past it (DublShot), or the column ahead (TrplShot). A panel
-- that can't be dug (not solid, or something on it) only cracks. 21 ticks
-- in all.

local lockon = require("../../rules/lockon")
local regions = require("../../lib/regions")
local shot = require("./shot")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for the three chips' digs.
local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local DIG, RECOVER = 0x0, 0x4

local SOUND = asset.sound("bblstar1")
-- The tick (timer value) the shots fly.
local THROW_AT = 0x0D
-- Each shot's crossing budget: it flies across at most 5 panel centers.
local CENTERS = 5

-- A chip's dig: the shots leave from each panel of `region`
-- (`dword_80ECA08`'s row), from the panel ahead.
local function dig(region: Region): Action
    -- `sub_80EC97C`: dig, fling the shots on the timer's 0xD, then recover.
    local function digging(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0C)
            battle.play_sound(SOUND)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0x0F
            me.step_init = 4
        end
        if s.timer == THROW_AT then
            local dir = me.facing
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            local x, y = me.panel_x + dir, me.panel_y
            for _, p in region.panels or {} do
                shot.spawn(me, x + dir * p[1], y + p[2], me.attack_element, damage, CENTERS)
            end
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            s.timer = 5
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80ECA0C`: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80EC960`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == DIG then
            digging(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("CrakShot phase %#x reads past its phase table", step))
        end
    end

    return define.action { state = STATE, args = { region = region }, update = update }
end

return {
    define.chip {
        id = "crakshot",
        codes = { "A", "G", "T", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 4, damage = 60, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 88, index = 88, sort = 114 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.crakshot },
        action = dig(regions.single),
    },
    define.chip {
        id = "dublshot",
        codes = { "C", "R", "U", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 8, damage = 60, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 89, index = 89, sort = 132 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.dublshot },
        action = dig(regions.long),
    },
    define.chip {
        id = "trplshot",
        codes = { "J", "L", "V", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 12, damage = 100, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 90, index = 90, sort = 447 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundels1 },
        action = dig(regions.wide),
    },
}
