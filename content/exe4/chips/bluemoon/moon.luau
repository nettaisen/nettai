--!strict
-- BlueMoon's moon (EXE4's attack object #0x86, 0x080DDFEC; EXE4's own): it rises over its user's panel and shines;
-- after 21 ticks it sends out its ray (./ray) at the panel three columns ahead and holds 91 ticks; then 11 more, and
-- its controller (../../lib/sun_and_moon) moves on and it goes.

local ray = require("./ray")
local sun_and_moon = require("../../lib/sun_and_moon")

local moon = {}

export type State = {
    -- The controller waiting for it (its +0x4C: the controller's flag).
    controller: Object?,
}

local SPRITE = asset.sprite("blue-moon")
local SOUND_APPEAR = asset.sound("tops")
local SOUND_SHINE = asset.sound("sun-moon-shine")

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t < 0
end

-- Action 0 (0x080DE06C): its sound; 45 ticks shining (its sound 28 ticks in); then action 4.
local function appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.timer = 44
        me.phase_init = 4
    end
    if me.timer == 17 then
        battle.play_sound(SOUND_SHINE)
    end
    if ran_out(me) then
        me:set_action(4)
    end
end

-- Action 4 (0x080DE0A2), by phase: 21 ticks (0x080DE0C0); animation 1 and its ray, 91 ticks (0x080DE0DE); animation
-- 2, 11 ticks, then the controller goes on and it goes (0x080DE10C).
local function shine(me: Object)
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            me.timer = 20
            me.phase_init = 4
        end
        if ran_out(me) then
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        if me.phase_init == 0 then
            me.anim = 1
            ray.spawn(me, me.panel_x, me.panel_y, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
            me.timer = 90
            me.phase_init = 4
        end
        if ran_out(me) then
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        if me.phase_init == 0 then
            me.anim = 2
            me.timer = 10
            me.phase_init = 4
        end
        if ran_out(me) then
            sun_and_moon.done((me.state :: State).controller)
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("BlueMoon's moon: phase %#x reads past its table (0x080DE0B4)", phase))
    end
end

-- 0x080DE052: by action.
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        appear(me)
    elseif a == 4 then
        shine(me)
    else
        error(string.format("BlueMoon's moon: action %#x reads past its table (0x080DE064)", a))
    end
end

-- 0x080DE010: its sprite (no shadow), from animation 0, turned to its side's way, no element, on its panel's ground.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.element = 0
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_lifecycle("update")
    tick(me)
end

local kind = new.kind {
    id = "bluemoon/moon",
    pool = "attack",
    state = { controller = "object" },
    -- 0x080DDFEC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DE134: the moon on the controller's panel, with its element and the damage word, of the controller's side
-- alone; it acts while dimmed. (Its position is the spawner's registers, which its init overwrites: the row, the
-- element and 0.)
function moon.spawn(controller: Object, damage: number): Object?
    local o = battle.spawn(kind, Vec3.new(controller.panel_y, controller.element, 0))
    if o then
        o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
        o.element = controller.element
        o:set_damage_word(damage)
        o.alliance = controller.alliance
        ;(o.state :: State).controller = controller
        o.run_while_dimmed = true
    end
    return o
end

return moon
