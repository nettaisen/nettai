--!strict
-- Phoenix's fireball (EXE5's attack object #0xC5, 0x080DF490; EXE6's is DustCross Beast's junk, rewritten): from over
-- the phoenix's column it falls in a straight line onto its target panel in 8 ticks, highlighting it; it hits there
-- for a tick (a fire hit), then bursts (./blaze), shaking the camera, and goes. It is gone at once when the battle is
-- over.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local blaze = require("./blaze")

local fireball = {}

export type State = {
    -- Its target panel, x | y << 4 (its Param2).
    target: number,
    -- Where it started (+0x60..+0x68) and its target's center (+0x6C, +0x70).
    start: Vec3,
    target_x: number,
    target_y: number,
}

local SPRITE = asset.sprite("meteor")
local SOUND_THROW = asset.sound("justice-one")
local SOUND_BURST = asset.sound("form-change")
local FALL = 8
local HIT_MOD = 3

-- `sub_80DCACC`: a and b, `t` eighths of the way to their ends (each step's division truncating).
local function lerp(a: number, b: number, a1: number, b1: number, t: number): (number, number)
    return a + int.tdiv(int.i32((a1 - a) * t), FALL), b + int.tdiv(int.i32((b1 - b) * t), FALL)
end

-- Action 0 (0x080DF56C): shown, where it started.
local function show(me: Object)
    local s = me.state :: State
    me.visible = true
    me.timer = 0
    local x, z = lerp(s.start.x, s.start.z, s.target_x, 0, 0)
    me.pos = Vec3.new(x, me.pos.y, z)
    me.action = 4
end

-- Action 4 (0x080DF594): its target highlighted; the fall's 8 ticks; then its hit on the target.
local function fall(me: Object)
    local s = me.state :: State
    field.highlight(me.panel_x, me.panel_y)
    local t = int.u16(me.timer + 1)
    me.timer = t
    if t > FALL then
        me.collision:set_region(regions.single)
        me.action = 8
        return
    end
    local x, z = lerp(s.start.x, s.start.z, s.target_x, 0, t)
    local y = lerp(s.start.y, s.start.z, s.target_y, 0, t)
    me.pos = Vec3.new(x, y, z)
end

-- Action 8 (0x080DF5DC): the blaze, its sound, the camera shaking; gone.
local function burst(me: Object)
    blaze.spawn(me, me.panel_x, me.panel_y)
    battle.play_sound(SOUND_BURST)
    battle.shake_camera(2, 0xA)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = show, [4] = fall, [8] = burst }

-- 0x080DF51E: its hits resolved (its hit off once it hit), gone when the battle is over; else its action.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me.lifecycle = "destroy"
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("Phoenix's fireball: action %#x reads past its table (0x080DF560)", me.action))
    end
    action(me)
    me:update_sprite()
    c:present()
end

-- 0x080DF4B0: its sprite (no shadow), its start and its target's center kept, on its target panel, its hit set up
-- but nowhere yet; its sound; then its first update.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    s.start = me.pos
    me.panel_x, me.panel_y = bit32.band(s.target, 7), bit32.rshift(s.target, 4)
    s.target_x, s.target_y = field.center(me.panel_x, me.panel_y)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        -- The game branches into its fall's code (0x080DF5B4) with the wrong stack.
        error("Phoenix's fireball got no collision data: the game goes on with a broken stack (0x080DF4FA)")
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
    battle.play_sound(SOUND_THROW)
    tick(me)
end

fireball.kind = new.kind {
    id = "phoenix/fireball",
    pool = "attack",
    state = { target = "u8", start = "vec3", target_x = "i32", target_y = "i32" },
    -- 0x080DF490.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- 0x080DF61A: a fireball of `owner`'s at `pos` for the panel `target` (x | y << 4), with the damage word, a fire
-- attack, running while dimmed.
function fireball.spawn(owner: Object, pos: Vec3, target: number, damage: number): Object?
    local o = battle.spawn(fireball.kind, pos)
    if o then
        (o.state :: State).target = target
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
        o.element = 1
        o.run_while_dimmed = true
    end
    return o
end

return fireball
