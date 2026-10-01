--!strict
-- MagCoil (the original's action 0x44, `sub_80EE6FC`): the navi holds out
-- its coil and, a tick later, sets a magnet (attack object #0x95,
-- ./magnet) on the panel ahead that pulls the enemies in the rows beside
-- it toward its row for 60 ticks; the navi holds 10 ticks, then puts the
-- coil away over 7.

local attachment = require("../../objects/attachment/attachment")
local magnet = require("./magnet")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10), and the hold
    -- (AIAttackVars+0x12).
    timer: number,
    hold: number,
}

local STATE = { timer = "u16", hold = "u16" }

-- The phases, on the attack's step.
local RAISE, SET, LOWER = 0x0, 0x4, 0x8

local ANIM_RAISE = 0x0A
local SOUND = asset.sound("elec-pulse")
-- The coil the navi holds (the original's attachment row 0x2C).
local COIL = attachment.look { sprite = asset.sprite("magnet-coil"), palette = 0, attach_point = 0x0E }
local HOLD, LOWER_TICKS = 0x0A, 6

-- The coil's animation steps. Without a coil (its pool was full) there is
-- nothing to step.
local function step_coil(me: Object, by: number)
    local coil = me.overlay
    if coil then
        coil.anim += by
    end
end

-- MagCoil's action: a magnet that stands `ticks` (the original's first
-- parameter).
local function mag_coil(spec: MagCoilSpec): Action
    -- `sub_80EE71C`: the coil comes out.
    local function raise(me: Object, s: State)
        me:set_animation(ANIM_RAISE)
        me:open_counter_window()
        attachment.attach(me, COIL, "overlay")
        me:set_status("using_action", true)
        s.timer = 1
        me.step = SET
        me.step_init = 0
    end

    -- `sub_80EE748`: a tick later the magnet goes out; hold.
    local function set(me: Object, s: State)
        if me.step_init == 0 then
            local t = s.timer - 1
            s.timer = t
            if t > 0 then
                return
            end
            step_coil(me, 1)
            s.hold = HOLD
            me.step_init = 4
            magnet.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, spec.ticks)
            battle.play_sound(SOUND)
        end
        local h = s.hold - 1
        s.hold = h
        if h > 0 then
            return
        end
        me.step = LOWER
        me.step_init = 0
    end

    -- `sub_80EE7A0`: put the coil away; 7 ticks later, back to idle.
    local function lower(me: Object, s: State)
        if me.step_init == 0 then
            step_coil(me, -1)
            s.timer = LOWER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    -- `sub_80EE6FC`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == RAISE then
            raise(me, s)
        elseif step == SET then
            set(me, s)
        elseif step == LOWER then
            lower(me, s)
        else
            error(string.format("MagCoil phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = STATE, args = spec, update = update }
end

return define.chip {
    id = "magcoil",
    name = "MagCoil",
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 14, damage = 0, hit_param = 30, lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { "free_slot_in" },
    library = { number = 185, index = 185, sort = 308 },
    slot_in_limit = 3,
    action = mag_coil { ticks = 0x3C },
}
