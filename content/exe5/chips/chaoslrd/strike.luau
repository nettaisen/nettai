--!strict
-- The chaos strike (EXE5's attack object #0x82, 0x080D67D8; EXE5's own, its dispatch EXE6's `sub_80D5740`): a ball of
-- dark energy that gathers in front of its thrower (46 pixels ahead, 50 up), changes look as it grows (0x080D6A1C's
-- rows: the charge glow, the orb, then two of Bass's animations), picks its target panel 15 ticks before it flies and
-- flies there in 16 ticks, where it bursts: explosions over the 3x3 around the panel (the panel bursts), a hit there
-- (no spark, hitting navis, thrown and breaking, hit modifier 3), the panels around it broken, the camera shaking and
-- the screen flashing. ChaosLrd's (./navi, its Param1 1) strikes the middle of the other side's area. (Its Param1 0
-- is the Bass AI's, 0x08102B1E, which no netbattle reaches: it grows 75 ticks, its looks at 58, 50 and 45 left, and
-- strikes the panel of the object it was thrown at, highlighting it every other 4 ticks, in the napalm fire's burst
-- (the effect table's row 0x6B), cracking the panels around it that may crack.)

local collision = require("../../rules/collision")
local panel_bursts = require("../../objects/panel_bursts")
local regions = require("@exelib/regions")
local trajectory = require("../../lib/trajectory")

export type State = {
    -- Its target panel (Param3, Param4; 0 until picked).
    x: number,
    y: number,
}

local strike = {}

-- Its looks as it grows (0x080D6A4C's rows 0 to 3: sprite, animation, palette): the charge glow, the orb, and two of
-- Bass's animations.
type Look = { sprite: SpriteAsset, anim: number, palette: number }
local GLOW: Look = { sprite = asset.sprite("charge-glow"), anim = 0, palette = 2 }
local ORB: Look = { sprite = asset.sprite("chaos-orb"), anim = 0, palette = 0 }
local GROWN: Look = { sprite = asset.sprite("bass"), anim = 0x27, palette = 0 }
local FULL: Look = { sprite = asset.sprite("bass"), anim = 0x28, palette = 0 }
local SOUND_GROW = asset.sound("point-rise")
local SOUND_THROW = asset.sound("windrack")
local SOUND_BLAST = asset.sound("chaos-blast")
-- The panel bursts' row (0x080E3164) its Param1 + 5 names: 6 (the effect table's row 0x24), with no sound.
local BURST = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 }
-- The panels it breaks around its target (0x080162A4's list 15: the 3x3, x toward its facing).
local AROUND: { { number } } = {
    { 0, 0 },
    { 0, -1 },
    { 0, 1 },
    { 1, 0 },
    { -1, 0 },
    { 1, -1 },
    { -1, 1 },
    { 1, 1 },
    { -1, -1 },
}
local SOLID = 0x10
-- Its flight (to -1), and its height of 8 pixels it was meant to come down to (the code divides its height alone).
local FLIGHT = 0x0F
local EIGHT = 0x8_0000

-- A halfword add to a 16.16 word's whole part (`ldrh`/`strh` at +2).
local function whole_add(v: number, n: number): number
    local hi = bit32.band(bit32.rshift(v, 16) + n, 0xFFFF)
    return int.i32(bit32.bor(bit32.band(v, 0xFFFF), bit32.lshift(hi, 16)))
end

-- 0x080D6A1C: look `l` (its sprite, no shadow, its animation as loaded, its palette).
local function look(me: Object, l: Look)
    local sprite = me.sprite
    sprite:load(l.sprite)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = l.anim, l.anim
    sprite:set_animation(l.anim)
    sprite.palette = l.palette
end

-- Action 0 (0x080D686C): growing (the orb, Bass's two looks at 88, 80 and 75 left), its target picked at 15 left;
-- when its time runs out it is shown and thrown at its target's panel.
local function grow(me: Object)
    local s = me.state :: State
    local t = me.timer
    if t == 0x58 then
        look(me, ORB)
        me.pos = Vec3.new(me.pos.x, me.pos.y, whole_add(me.pos.z, 18))
    elseif t == 0x50 then
        look(me, GROWN)
        battle.play_sound(SOUND_GROW)
    else
        if t == 0x4B then
            look(me, FULL)
            me.pos = Vec3.new(me.pos.x, me.pos.y, whole_add(me.pos.z, 12))
        end
        if t == 0x0F then
            -- The middle of the other side's area: column 5 for a side facing right, 2 for one facing left.
            s.x = bit32.bxor(bit32.bxor(me.alliance, me.flip), 1) * 3 + 2
            s.y = 2
        end
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    me.visible = true
    local p = me.pos
    local cx, cy = field.center(s.x, s.y)
    local vx = trajectory.div(int.i32(cx - p.x), FLIGHT)
    local vy = trajectory.div(int.i32(cy + EIGHT - p.y), FLIGHT)
    local vz = trajectory.div(p.z, FLIGHT)
    me.vel = Vec3.new(vx, vy, vz)
    me.timer = FLIGHT
    battle.play_sound(SOUND_THROW)
    me.action = 4
end

-- 0x080D696E: it bursts on its target panel.
local function burst(me: Object)
    local s = me.state :: State
    panel_bursts.spawn(me, regions.square, 1, 0, BURST, nil, s.x, s.y)
    -- `object_spawnCollisionRegion`, running while dimmed as it does.
    local box = battle.hitbox(me, {
        panel_x = s.x,
        panel_y = s.y,
        element = 0,
        z = 0,
        region = regions.square,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.thrown_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = me.run_while_dimmed
    end
    -- 0x081029A6: the solid panels around it break.
    for _, d in AROUND do
        local x, y = s.x + d[1] * me.facing, s.y + d[2]
        if bit32.btest(field.flags(x, y), SOLID) then
            field.break_panel(x, y)
        end
    end
    battle.shake_camera(3, 0x14)
    battle.play_sound(SOUND_BLAST)
    battle.palette_flash(1, 4, true, false)
    me:set_lifecycle("destroy")
end

-- Action 4 (0x080D6948): it flies, its height coming down, for 16 ticks; then it bursts.
local function fly(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z - v.z))
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        burst(me)
    end
end

local ACTIONS = { [0] = grow, [4] = fly }

-- 0x080D6844: at the battle's end it goes.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("the chaos strike: action %#x reads past its table (0x080D6864)", me.action))
    end
    action(me)
end

-- 0x080D67FC: the glow, shown, facing its way; it grows 105 ticks; 46 pixels ahead of its thrower, 50 up and 8 down
-- the field; its first update.
local function init(me: Object)
    look(me, GLOW)
    me.visible = true
    me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = 0x69
    local p = me.pos
    me.pos = Vec3.new(whole_add(p.x, me.facing * 0x2E), whole_add(p.y, 8), whole_add(p.z, 0x32))
    me:set_lifecycle("update")
    tick(me)
end

strike.kind = new.kind {
    id = "chaoslrd/strike",
    pool = "attack",
    state = { x = "u8", y = "u8" },
    -- 0x080D67D8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080D69E6: a strike of `owner`'s side at `pos` (its thrower's position) with the damage word, running while
-- dimmed (ChaosLrd's: Param1 1, no target).
function strike.spawn(owner: Object, pos: Vec3, damage: number): Object?
    local o = battle.spawn(strike.kind, pos)
    if o then
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return strike
