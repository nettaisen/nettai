--!strict
-- A panel PanlSht tears up and shoots (EXE4's attack object #0x7B, 0x080DCAA4: EXE6's `sub_80D4A28`, which no EXE6
-- chip spawns, with EXE4's look): its sheet (0c-38) in the palette of the side whose panel it was, as the viewer sees
-- it; it breaks the panel it rises from if nothing stands on it, a sound (0x1C6); rises 2 pixels a tick to 16 up, waits its ticks, then flies
-- forward 10 pixels a tick (0xBF) until it hits something or leaves the field. Its hit is live from its first tick:
-- an attack at navis, hit modifier 3, spark 6. Gone, it lets its controller's flag for it go.

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")
local sparks = require("../../lib/sparks")

local panel = {}

export type State = {
    -- The controller waiting for it (+0x4C points at its flag for this panel).
    controller: Object?,
    -- Its ticks before it flies (Param1).
    wait: number,
}

local SPRITE = asset.sprite("panel-shot")
local TEAR_SOUND = asset.sound("panel-lift")
local SHOT_SOUND = asset.sound("wood-strike")
local HIT_MOD = 3
-- Rising 2 pixels a tick to 16 up; flying 10 pixels a tick; gone once its X is 160 pixels or more off the field's
-- middle.
local RISE, TOP, SPEED, REACH = 0x2_0000, 16, 10, 0xA0

local RISING, WAITING, FLYING = 0x0, 0x4, 0x8

-- The controller's panels out (its flags, one a panel: 0x080E7BA8 waits while any is set).
export type ControllerState = { out: number }

-- Its controller's flag for it goes.
local function let_go(me: Object)
    local c = (me.state :: State).controller
    if c then
        local cs = c.state :: ControllerState
        cs.out -= 1
    end
end

-- Action 0 (0x080DCBB0): 2 pixels up a tick, to 16.
local function rise(me: Object)
    local p = me.pos
    local z = p.z + RISE
    if int.asr(z, 16) >= TOP then
        me.pos = Vec3.new(p.x, p.y, TOP * 0x10000)
        me:set_action(WAITING)
        return
    end
    me.pos = Vec3.new(p.x, p.y, z)
end

-- Action 4 (0x080DCBD2): its ticks (its first parameter); then it sets off with a sound.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = (me.state :: State).wait
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    battle.play_sound(SHOT_SOUND)
    me:set_action(FLYING)
end

-- Action 8 (0x080DCBFA): forward; gone past the field.
local function fly(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if int.u32(int.asr(me.pos.x, 16) + REACH) >= 2 * REACH then
        me.collision:set_region(nil)
        let_go(me)
        me:set_lifecycle("destroy")
    end
end

-- 0x080DCB66: resolve last tick's hits; a hit ends it, hidden; else its action, and present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        me.visible = false
        c:set_region(nil)
        let_go(me)
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == RISING then
        rise(me)
    elseif a == WAITING then
        wait(me)
    elseif a == FLYING then
        fly(me)
    else
        error(string.format("PanlSht's panel: action %#x reads past its table (0x080DCBA4)", a))
    end
    c:present()
end

-- 0x080DCAC8: its sheet, shown, facing its way, on its panel, flying forward; the panel under it broken; its
-- palette its panel's side's as the viewer sees it; its collision; the sound; its first tick at once.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_animation(0)
    sprite:step()
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local v = me.vel
    me.vel = Vec3.new(panels.side_direction(me.alliance) * SPEED * 0x10000, v.y, v.z)
    field.break_empty(me.panel_x, me.panel_y)
    local p = field.panel(me.panel_x, me.panel_y)
    if p then
        sprite.palette = bit32.bxor(p.alliance, battle.local_side())
        if not me:create_collision() then
            me:free()
            return
        end
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.heavy)
    me.collision:present()
    battle.play_sound(TEAR_SOUND)
    me:set_lifecycle("update")
    tick(me)
end

panel.kind = new.kind {
    id = "panlsht/panel",
    pool = "attack",
    state = { controller = "object", wait = "u8" },
    -- 0x080DCAA4: by state, then its sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DCC2A: a panel of `controller`'s on (x, y), of its element and with the damage word, waiting `wait` ticks
-- before it flies, running while dimmed; it holds a flag of the controller's and sets it. (Its position is the
-- spawner's registers until its init places it: the row and the element.)
function panel.spawn(controller: Object, x: number, y: number, damage: number, wait: number): Object?
    local o = battle.spawn(panel.kind, Vec3.new(y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = controller.element
    o:set_damage_word(damage)
    o.alliance = controller.alliance
    o.run_while_dimmed = true
    local s = o.state :: State
    s.controller, s.wait = controller, wait
    local cs = controller.state :: ControllerState
    cs.out += 1
    return o
end

return panel
