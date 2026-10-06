--!strict
-- NumTrap's dimming controller (EXE5's effect object #0x40, 0x080E4D68; EXE5's own, on the dimming chips' skeleton):
-- the usual dimming phases (@exelib/dimming), with an effect (0x080E4DAC) in two parts. After 30 ticks it looks at
-- the panel two ahead of its user's: unless that is a panel of the other side's that can be stood on, with nothing
-- on it, nothing happens and the screen undims. Else its user holds his animation 12 for 70 ticks, with a sound, and
-- 20 ticks in the trap (./trap) is set on that panel, its damage the chip's and the Atk+ bonus; then he stands
-- again and the undim.

local dimming_chips = require("@exelib/dimming")
local trap = require("./trap")

export type State = {
    -- The Atk+ / cross bonus (object +0x32).
    bonus: number,
}

local controller = {}

-- The effect's parts (0x080E4DC0), on the phase.
local LOOK, SET = 0x0, 0x4
local LOOK_TICKS, SET_TICKS, TRAP_AT = 30, 70, 50
-- The panels ahead of the user the trap goes.
local AHEAD = 2
-- The user's animation while he sets it.
local ANIM_SET = 12
local SOUND = asset.sound("hockey-puck")
-- A panel of the other side's that can be stood on, with nothing on it, by the controller's side (0x080E4E28).
local FREE = {
    [0] = { require = 0x30, forbid = 0x0F88_0080 },
    [1] = { require = 0x10, forbid = 0x0F88_00A0 },
}

-- 0x080E4DC8: 30 ticks on its user's panel; then the panel two ahead: free, the trap goes there; else the undim.
local function look(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
        me.timer = LOOK_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    local x, y = me.panel_x + AHEAD * me.facing, me.panel_y
    me.future_panel_x, me.future_panel_y = x, y
    local free = FREE[me.alliance]
    if field.check(x, y, free.require, free.forbid) then
        me.phase, me.phase_init = SET, 0
    else
        dimming_chips.done(me)
    end
end

-- 0x080E4E38: the user's pose and the sound; the trap when 50 ticks are left; after 71 the user stands again, and
-- the undim.
local function set(me: Object, s: State)
    local user = me.related1 :: Object
    if me.phase_init == 0 then
        me.phase_init = 4
        user.anim = ANIM_SET
        battle.play_sound(SOUND)
        me.timer = SET_TICKS
    else
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left == TRAP_AT then
            local damage = dimming_chips.damage_plus(me, s.bonus)
            trap.spawn(me, me.future_panel_x, me.future_panel_y, me.element, damage, s.bonus)
        elseif left < 0 then
            user.anim = 0
            dimming_chips.done(me)
        end
    end
    user:update_sprite_while_dimmed()
end

-- 0x080E4DAC.
local function effect(me: Object)
    local phase = me.phase
    if phase == LOOK then
        look(me)
    elseif phase == SET then
        set(me, me.state :: State)
    else
        error(string.format("NumTrap's controller: phase %#x of its effect reads past its table (0x080E4DC0)", phase))
    end
end

controller.kind = new.kind {
    id = "numberman/numtrap/controller",
    pool = "effect",
    state = { bonus = "u16" },
    -- 0x080E4D68.
    update = dimming_chips.phases { name = "NumTrap", effect = effect },
}

-- 0x080E4E98: the controller for `user`, on his panel, of his side and facing, with the attack's element and
-- damage word and the chip's bonus. (Its position is the spawner's registers: the row, the element and the bonus
-- as its height's upper half.)
function controller.spawn(user: Object, element: number, damage: number, bonus: number): Object?
    local o = battle.spawn(controller.kind, Vec3.new(user.panel_y, element, int.i32(bonus * 0x1_0000)))
    if o then
        (o.state :: State).bonus = bonus
        o.panel_x, o.panel_y = user.panel_x, user.panel_y
        o.element = element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
    end
    return o
end

return controller
