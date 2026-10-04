--!strict
-- The chaos strike (BN5's attack object #0x82, 0x080D67D8; BN5's own, its dispatch BN6's `sub_80D5740`): a ball of
-- dark energy that gathers in front of its thrower (46 pixels ahead, 50 up), changes look as it grows (0x080D6A1C's
-- rows: the charge glow, the orb, then two of Bass's animations), picks its target panel 15 ticks before it flies and
-- flies there in 16 ticks, where it bursts: explosions over the 3x3 around the panel (the panel bursts), a hit there
-- (no spark, hitting navis, thrown and breaking, hit modifier 3), the panels around it broken, the camera shaking and
-- the screen flashing. ChaosLrd's (./navi, its Param1 1) waits longer and strikes the middle of the other side's
-- area; the Bass AI's (Param1 0, 0x08102B1E, which no netbattle reaches) strikes the panel of the object it was
-- thrown at, highlighting it every other 4 ticks, and only cracks the panels.

local collision = require("../../rules/collision")
local panel_bursts = require("../../objects/panel-bursts/panel_bursts")
local regions = require("@exelib/regions")
local trajectory = require("../../lib/trajectory")

export type State = {
    -- ChaosLrd's (Param1 1), or the Bass AI's (0).
    lord: boolean,
    -- The object the Bass AI's strikes (+0x4C).
    target: Object?,
    -- Its target panel (Param3, Param4; 0 until picked).
    x: number,
    y: number,
}

local strike = {}

-- Its looks (0x080D6A4C: sprite, animation, palette).
type Look = { sprite: SpriteAsset, anim: number, palette: number }
local LOOKS: { [number]: Look } = {
    [0] = { sprite = asset.sprite("bn5:charge-glow"), anim = 0, palette = 2 },
    [1] = { sprite = asset.sprite("bn5:chaos-orb"), anim = 0, palette = 0 },
    [2] = { sprite = asset.sprite("bn5:bass"), anim = 0x27, palette = 0 },
    [3] = { sprite = asset.sprite("bn5:bass"), anim = 0x28, palette = 0 },
}
local SOUND_GROW = asset.sound("bn5:point-rise")
local SOUND_THROW = asset.sound("bn5:windrack")
local SOUND_BLAST = asset.sound("bn5:chaos-blast")
-- The panel bursts' rows (0x080E3164) its Param1 + 5 names: the Bass AI's 5 (the effect table's row 0x6B), ChaosLrd's
-- 6 (row 0x24); neither has a sound.
local BURST_AI = define.effect { sprite = asset.sprite("bn5:napalm-fire"), anim = 0, palette = 1 }
local BURST_LORD = define.effect { sprite = asset.sprite("bn5:explosion"), anim = 0, palette = 1 }
-- The panels it breaks or cracks around its target (0x080162A4's list 15: the 3x3, x toward its facing).
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
local SOLID, NO_CRACK = 0x10, 0x40
-- Its flight (to -1), and its height of 8 pixels it was meant to come down to (the code divides its height alone).
local FLIGHT = 0x0F
local EIGHT = 0x8_0000

-- A halfword add to a 16.16 word's whole part (`ldrh`/`strh` at +2).
local function whole_add(v: number, n: number): number
    local hi = bit32.band(bit32.rshift(v, 16) + n, 0xFFFF)
    return int.i32(bit32.bor(bit32.band(v, 0xFFFF), bit32.lshift(hi, 16)))
end

-- 0x080D6A1C: look `n` (its sprite, no shadow, its animation as loaded, its palette).
local function look(me: Object, n: number)
    local l = LOOKS[n]
    local sprite = me.sprite
    sprite:load(l.sprite)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = l.anim, l.anim
    sprite:set_animation(l.anim)
    sprite.palette = l.palette
end

-- 0x080D69FA: the Bass AI's highlights its target panel when bit 2 of its second timer is clear (counting down).
local function highlight(me: Object)
    local s = me.state :: State
    if s.lord then
        return
    end
    local t = me.timer2
    me.timer2 = int.u16(t - 1)
    if bit32.band(t, 4) == 0 and s.x ~= 0 then
        field.highlight(s.x, s.y)
    end
end

-- Action 0 (0x080D686C): growing (ChaosLrd's: looks 1, 2 and 3 at 88, 80 and 75 left; the Bass AI's at 58, 50, 45),
-- its target picked at 15 left; when its time runs out it is shown and thrown at its target's panel.
local function grow(me: Object)
    local s = me.state :: State
    local t = me.timer
    if t == (if s.lord then 0x58 else 0x3A) then
        look(me, 1)
        me.pos = Vec3.new(me.pos.x, me.pos.y, whole_add(me.pos.z, 18))
    elseif t == (if s.lord then 0x50 else 0x32) then
        look(me, 2)
        battle.play_sound(SOUND_GROW)
    else
        if t == (if s.lord then 0x4B else 0x2D) then
            look(me, 3)
            me.pos = Vec3.new(me.pos.x, me.pos.y, whole_add(me.pos.z, 12))
        end
        if t == 0x0F then
            if s.lord then
                -- The middle of the other side's area: column 5 for a side facing right, 2 for one facing left.
                s.x = bit32.bxor(bit32.bxor(me.alliance, me.flip), 1) * 3 + 2
                s.y = 2
            else
                local target = s.target :: Object
                s.x, s.y = target.panel_x, target.panel_y
            end
        end
    end
    highlight(me)
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
    panel_bursts.spawn(me, regions.square, 1, 0, if s.lord then BURST_LORD else BURST_AI, nil, s.x, s.y)
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
    -- 0x081029A6: the panels around it: ChaosLrd's breaks the solid ones, the Bass AI's cracks those that may crack.
    for _, d in AROUND do
        local x, y = s.x + d[1] * me.facing, s.y + d[2]
        local flags = field.flags(x, y)
        if s.lord then
            if bit32.btest(flags, SOLID) then
                field.break_panel(x, y)
            end
        elseif not bit32.btest(flags, NO_CRACK) then
            field.crack(x, y)
        end
    end
    battle.shake_camera(3, 0x14)
    battle.play_sound(SOUND_BLAST)
    battle.palette_flash(1, 4, true, false)
    me:set_lifecycle("destroy")
end

-- Action 4 (0x080D6948): it flies, its height coming down, for 16 ticks; then it bursts.
local function fly(me: Object)
    highlight(me)
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

-- 0x080D67FC: look 0, shown, facing its way; ChaosLrd's grows 105 ticks, the Bass AI's 75; 46 pixels ahead of its
-- thrower, 50 up and 8 down the field; its first update.
local function init(me: Object)
    local s = me.state :: State
    look(me, 0)
    me.visible = true
    me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = if s.lord then 0x69 else 0x4B
    local p = me.pos
    me.pos = Vec3.new(whole_add(p.x, me.facing * 0x2E), whole_add(p.y, 8), whole_add(p.z, 0x32))
    me:set_lifecycle("update")
    tick(me)
end

strike.kind = define.kind {
    id = "bn5:chaoslrd/strike",
    pool = "attack",
    state = { lord = "bool", target = "object", x = "u8", y = "u8" },
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

-- 0x080D69C8 (0x080D69E6 with `while_dimmed`): a strike of `owner`'s side at `pos` (its thrower's position) with the
-- damage word, ChaosLrd's (`lord`) or the Bass AI's at `target`.
function strike.spawn(owner: Object, pos: Vec3, lord: boolean, target: Object?, damage: number, while_dimmed: boolean): Object?
    local o = battle.spawn(strike.kind, pos)
    if o then
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.lord, s.target = lord, target
        o.related1 = target
        o.run_while_dimmed = while_dimmed
    end
    return o
end

return strike
