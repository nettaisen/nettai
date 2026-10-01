--!strict
-- BblStar1, BblStar2 and BblStar3 (the original's action 0x2D,
-- `sub_80ED2F8`): the navi blows a bubble star (star.luau) from the panel
-- ahead on the third tick; it drifts forward along a wave and traps what
-- it hits in a bubble. The three differ in the star's speed and palette.
-- 32 ticks in all.

local star = require("./star")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

-- One state for the three chips' blows.
local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local BLOW, RECOVER = 0x0, 0x4
-- The tick (timer value) the star is blown.
local BLOW_AT = 0x0D

local SOUND_BLOW = asset.sound("bblstar1")
local SOUND_STAR = asset.sound("hockey-puck")

-- What a chip's star is like (`byte_80CC65C`'s row and the palette, which
-- the original picks by the chip's first parameter): its speed, 16.16 a
-- tick, and its palette.
type Blow = { speed: number, palette: number }

-- A chip's blow.
local function blow(spec: Blow): Action
    -- `sub_80ED314`: blow the star on the timer's 0xD, then recover.
    local function blowing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(0x0C)
            battle.play_sound(SOUND_BLOW)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0x0F
            me.step_init = 4
        end
        if s.timer == BLOW_AT then
            -- The damage word (the hit parameter in its high half), plus
            -- the Atk+ bonus.
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            star.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, spec.speed, spec.palette)
            battle.play_sound(SOUND_STAR)
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            s.timer = 0x0F
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- `sub_80ECA0C`: 16 ticks, then back to idle.
    local function recover(me: Object, s: State)
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80ED2F8`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == BLOW then
            blowing(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("BblStar phase %#x reads past its phase table", step))
        end
    end

    return define.action { state = STATE, args = spec, update = update }
end

return {
    define.chip {
        id = "bblstar1",
        name = "BblStar1",
        codes = { "B", "E", "T" },
        element = "aqua", family = "aqua", class = "standard",
        rarity = 0, mb = 30, damage = 60, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 26, index = 26, sort = 32 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = blow { speed = 0x1_8000, palette = 0 },
    },
    define.chip {
        id = "bblstar2",
        name = "BblStar2",
        codes = { "C", "L", "V" },
        element = "aqua", family = "aqua", class = "standard",
        rarity = 1, mb = 38, damage = 80, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 27, index = 27, sort = 33 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = blow { speed = 0x2_4000, palette = 1 },
    },
    define.chip {
        id = "bblstar3",
        name = "BblStar3",
        codes = { "G", "R", "S" },
        element = "aqua", family = "aqua", class = "standard",
        rarity = 2, mb = 46, damage = 100, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 28, index = 28, sort = 34 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = blow { speed = 0x3_0000, palette = 2 },
    },
}
