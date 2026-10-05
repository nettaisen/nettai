--!strict
-- Bass's shot (EXE5's attack object #0x72, 0x080D4214; EXE6's panel strike descends from it): it highlights its
-- panel for 10 ticks, then, on a solid panel, shows Bass's burst (his sprite's animation 32) and hits the panel
-- for a tick; it goes when the burst ends. The navi chip's (its Param1 1) outlasts the battle's end.
--
-- Not ported: its Param1 0 (Bass the virus's, 0x081025CC): 20 ticks of warning blinking, hitting with no hit
-- modifier and no damage until it shows, any panel.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local regions = require("@exelib/regions")

local strike = {}

local SPRITE = asset.sprite("bass")
local SOUND = asset.sound("col-army-2")
local WARNING = 0xA

-- 0x080D4238: on its panel, its hit set up but nowhere yet.
local function init(me: Object)
    me:set_coordinates_from_panel()
    me.timer = WARNING
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 3)
    local c = me.collision
    c:set_hit_effect(sparks.plain)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
end

-- Action 0 (0x080D42DC): the warning; then, on a solid panel, the burst and its hit.
local function wait(me: Object)
    field.highlight(me.panel_x, me.panel_y)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    local c = me.collision
    if not field.solid(me.panel_x, me.panel_y) then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0x20, 0x20
    sprite:set_animation(0x20)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    c:set_region(regions.single)
    battle.play_sound(SOUND)
    me:set_action(4)
end

-- Action 4 (0x080D4380): its hit off after a tick; gone when the burst ends.
local function burst(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.collision:set_region(nil)
    end
    if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = wait, [4] = burst }

-- 0x080D429E: resolve the hits; act; register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("Bass's shot action %#x reads past its table (0x080D42D4)", me.action))
    end
    action(me)
    c:present()
end

strike.kind = define.kind {
    id = "bass/strike",
    pool = "attack",
    -- 0x080D4214.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080D43C2: Bass's shot on (x, y) with his damage word, running while dimmed.
function strike.spawn(bass: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(strike.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = bass.alliance, bass.flip
        o.run_while_dimmed = true
    end
    return o
end

return strike
