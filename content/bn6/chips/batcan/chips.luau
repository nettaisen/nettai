--!strict
-- BatCan1-4: the navi raises its bat cannon and fires an unseen shot
-- (shot.luau) from the panel ahead on the 7th tick; BatCan4 fires three,
-- 16 ticks apart, the last staggering harder (the original's action 0x19,
-- `sub_80EC0E6`; docs/engine/shot-chips.md §6).

local attachment = require("../../objects/attachment/attachment")
local lockon = require("../../rules/lockon")
local shot = require("./shot")

export type State = {
    -- The phase's count (AIAttackVars+0x10).
    timer: number,
}

-- One state for the four (their shots left are the attack's word at
-- AIAttackVars+0x12, the navi's `attack_count`).
local STATE = { timer = "u16" }

-- The phases, on the attack's step.
local COUNT, FIRE, RECOVER = 0x0, 0x4, 0x8

local ANIM_FIRE, ANIM_DONE = 9, 0
local SOUND = asset.sound("cannon")
-- The shot leaves on this tick of the count, and the phase ends on this
-- one; the recovery's ticks.
local SHOT_TICK, FIRE_TICKS, RECOVER_TICKS = 5, 0xF, 0x14
local SHOT_Z = 0x18_0000

-- The bat cannon (the original's attachment row 0xF: it sits at a fixed
-- offset by its holder, not at an attach point).
local BAT = attachment.look { sprite = asset.sprite("bat-cannon"), palette = 0, by_owner = true }

-- A chip's shots: each one's hit modifier, in the order they're fired
-- (the original's chip parameters, or BatCan4's `byte_80EC1BC`).
export type BatCanSpec = { shots: { number } }

local function batcan(spec: BatCanSpec): Action
    -- `sub_80EC104`: how many shots.
    local function count(me: Object)
        me.attack_count = #spec.shots
        me.step, me.step_init = FIRE, 0
    end

    -- `sub_80EC11C`: raise a new bat cannon (the form overlay restarts with
    -- the navi's animation); fire on the count's 5; at 0xF the next shot,
    -- or the recovery.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_FIRE)
            local overlay = me.related2
            if overlay then
                overlay.anim_loaded = 0xFF
            end
            me:open_counter_window()
            s.timer = 0
            local held = me.related1
            if held then
                held:set_lifecycle("destroy")
            end
            attachment.attach(me, BAT, "related")
            me:set_status("using_action", true)
            me.step_init = 4
            return
        end
        if s.timer == SHOT_TICK then
            battle.play_sound(SOUND)
            local hit_mod = spec.shots[#spec.shots - me.attack_count + 1]
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            shot.fire(me, me.panel_x + me.facing, me.panel_y, SHOT_Z, me.attack_element, damage, hit_mod)
        end
        local t = s.timer + 1
        s.timer = t
        if t == FIRE_TICKS then
            local left = me.attack_count - 1
            me.attack_count = left
            if left > 0 then
                me.step_init = 0
            else
                me.step, me.step_init = RECOVER, 0
            end
        end
    end

    -- `sub_80EC1C4`: 21 ticks, then the navi's plain animation, the cannon
    -- goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:set_animation(ANIM_DONE)
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    return define.action {
        state = STATE,
        args = spec,
        -- `sub_80EC0E6`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == COUNT then
                count(me)
            elseif step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("BatCan phase %#x reads past off_80EC0F8", step))
            end
        end,
    }
end

return {
    define.chip {
        id = "batcan1",
        codes = { "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 315, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 3 } },
    },
    define.chip {
        id = "batcan2",
        codes = { "S" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 316, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 3 } },
    },
    define.chip {
        id = "batcan3",
        codes = { "R" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 317, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 3 } },
    },
    define.chip {
        id = "batcan4",
        codes = { "Z" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 318, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 1, 1, 3 } },
    },
}
