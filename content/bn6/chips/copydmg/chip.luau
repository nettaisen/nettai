--!strict
-- CopyDmg (the original's action 0x23, `sub_80ECA34`): the navi raises its
-- buster and, on the next tick, fires its mark (mark.luau) along its row
-- from the panel ahead: the first navi it hits takes, for 180 ticks, the
-- damage its side took the tick before on top of its own. 5 ticks, then 11
-- of recovery.

local buster = require("../../lib/buster")
local lockon = require("../../rules/lockon")
local mark = require("./mark")

export type State = {
    -- Ticks into the shot, then ticks left (AIAttackVars+0x10).
    timer: number,
}

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

local SOUND = asset.sound("buster-shot")

-- `sub_80ECA50`: the buster up; the mark flies on the second tick.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        buster.raise_arm(me)
        me:open_counter_window()
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SOUND)
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        mark.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
    end
    local t = s.timer + 1
    s.timer = t
    if t > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80ECAA4`: 11 ticks, then the arm goes and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 0x0A
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

-- `sub_80ECA34`.
local function update(me: Object, s: State)
    local step = me.step
    if step == SHOOT then
        shoot(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("CopyDmg phase %#x reads past its phase table", step))
    end
end

return define.chip {
    id = "copydmg",
    codes = { "*" },
    element = "null", family = "cursor", class = "standard",
    rarity = 1, mb = 12, damage = 0, hit_param = 0, lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { "free_slot_in" },
    library = { number = 194, index = 194, sort = 110 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.cannon },
    action = define.action { state = { timer = "u16" }, update = update },
}
