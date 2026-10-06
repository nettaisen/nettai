--!strict
-- ShadowMan's shadow (EXE5's actor object #4, 0x080B9384; EXE5's own), as SplitUp sends it (./chip): one of him, in
-- his look, on the panel before the enemy SplitUp found. It appears (3 ticks), draws its sword and swings it for 21
-- ticks (the swing's sound; a slash over the panel ahead and the one past it on the swing's 10th tick, with its
-- effect), stands 20 ticks, and blinks out over 31. Nothing reaches it: SplitUp's has no collision region. It
-- waits while dimmed, and goes when the battle ends.
--
-- What SplitUp's never does, and isn't ported: a shadow that can be hit (its Param3 0: 1 HP, a body of its side;
-- before its slash it vanishes and is back on the panel before its owner's target, 0x080B9930, else on a random
-- panel, where it stands; its slash is Sword's, the panel ahead alone), and its other orders (its word at +0x68:
-- bit 1 throws a flame along its row, attack object #7 through 0x080C5492; bit 4 vanishes it; bit 8 moves it to a
-- random panel of its side's area, 0x080B98E4). Those orders come only to the two shadows the battle state keeps
-- (+0x94, +0x98), from ShadowMan's own actions 7 and 8 and his charge's variant 0, and only his auto battle's own
-- move puts shadows there (0x080F99EA, with a slot from 0x0800CEEE): the operation battle's ShadowMan, which
-- nettai doesn't build (docs/design/exe5-map.md §15.16).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local marker = require("../../lib/navi_chips/marker")
local regions = require("@exelib/regions")
local swords = require("../../lib/swords")

local shadow = {}

export type State = {
    -- Its sword (+0x6C: where its attachment is kept).
    held: Object?,
    -- Its orders (+0x68): the bits its owner sets.
    orders: number,
    -- Its hit ran out, or its work is done (+0x60): it takes no more hits.
    gone: boolean,
    -- The ticks it stands before the swing (Param1).
    wait: number,
    -- Its slash's damage word, with the Atk+ bonus (+0x7C).
    slash_damage: number,
}

local SPRITE = asset.sprite("shadowman")
local HP = 1
local HIT_MOD = 3
-- Its orders' bit SplitUp sets: slash.
local SLASH_ORDER = 2
local ANIM_APPEAR, ANIM_SWING = 3, 5
local APPEAR_TICKS, SWING_TICKS, SLASH_AT, STAND_TICKS, BLINK_TICKS = 3, 21, 12, 20, 30
-- Its sword's animation (the attachment's spawn word: row 3, animation 8, ShadowMan's own blade's).
local BLADE_ANIM = 8
local EFFECT_Z = 16 * 0x1_0000

-- Its actions (0x080B9494), and the slash's phases (0x080B9674).
local APPEAR, IDLE, GO, SLASH = 0, 4, 16, 20
local READY, SWING, STAND = 8, 12, 16

local function next_action(me: Object, action: number)
    me.action = action
    me.phase, me.phase_init = 0, 0
end

-- 0x080B94B0: it appears; 3 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_APPEAR
        me.timer = APPEAR_TICKS
        me.phase_init = 4
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        next_action(me, IDLE)
    end
end

-- 0x080B94D8: standing, it takes its orders. (Of the four the original tests, in the order 4, 1, 8, 2, SplitUp's
-- shadow gets the last alone.)
local function idle(me: Object, s: State)
    if me.phase_init == 0 then
        me.anim = 0
        me.phase_init = 4
    end
    if bit32.btest(s.orders, SLASH_ORDER) then
        s.orders = bit32.band(s.orders, bit32.bnot(SLASH_ORDER))
        next_action(me, SLASH)
    end
end

-- 0x080B9702: standing ready, its wait. (In the operation battle a marker over the panel ahead.)
local function ready(me: Object, s: State)
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = s.wait
        marker.spawn(me, me.panel_x + me.facing, me.panel_y, s.wait + 32, 1, false)
        me.phase_init = 4
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me.phase, me.phase_init = SWING, 0
    end
end

-- 0x080B973A: the swing, 21 ticks; the slash when 12 are left: the panel ahead and the one past it (region 2, a
-- slash with no spark, hit modifier 3, no element), under LongSwrd's slash (effect 0x17).
local function swing(me: Object, s: State)
    if me.phase_init == 0 then
        me.anim = ANIM_SWING
        me.timer = SWING_TICKS
        battle.play_sound(swords.swing)
        attachment.attach(me, swords.blade, "held", { anim = BLADE_ANIM })
        me.phase_init = 4
        return
    end
    if me.timer == SLASH_AT then
        local x, y = me.panel_x + me.facing, me.panel_y
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = 0,
            region = regions.long,
            hit_effect = nil,
            target = collision.attack,
            self_type = collision.slash,
            damage = bit32.band(s.slash_damage, 0xFFFF),
            stamina = bit32.rshift(s.slash_damage, 16),
            hit_mod = HIT_MOD,
        })
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, EFFECT_Z), swords.effects.long, bit32.bxor(me.alliance, me.flip))
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me.phase, me.phase_init = STAND, 0
    end
end

-- 0x080B97BE: 20 ticks; then it goes.
local function stand(me: Object, s: State)
    if me.phase_init == 0 then
        me.timer = STAND_TICKS
        me.phase_init = 4
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me.collision:set_region(nil)
        s.gone = true
        next_action(me, GO)
    end
end

-- 0x080B9660. (Its first phase, 0x080B9688, passes SplitUp's shadow straight on to its wait.)
local function slash(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        me.phase, me.phase_init = READY, 0
    elseif phase == READY then
        ready(me, s)
    elseif phase == SWING then
        swing(me, s)
    elseif phase == STAND then
        stand(me, s)
    else
        error(string.format("ShadowMan's shadow's slash: phase %#x isn't SplitUp's (0x080B9674)", phase))
    end
end

-- 0x080B9608: it blinks out, shown two ticks in four whoever looks, 31 ticks. (Its start takes it off the sides'
-- lists of alive actors, 0x08006BC2, which it was never on, and in the operation battle lets go of what tracks
-- it, 0x0802EFFA.)
local function go(me: Object)
    if me.phase_init == 0 then
        me.timer = BLINK_TICKS
        me.phase_init = 4
    end
    me.visible = not bit32.btest(me.timer, 2)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080B942C: shown, unless the viewer is blind to its side; until it goes, its hits (none reach SplitUp's; the
-- battle's end takes its HP); its action, unless the battle is dimmed.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me.visible = true
    me:hide_from_blind()
    if not s.gone then
        if battle.over() then
            me.hp = 0
        end
        if me:take_damage(0) < 0 then
            s.gone = true
            next_action(me, GO)
            c:set_region(nil)
        end
    end
    if not battle.dimmed() then
        local a = me.action
        if a == APPEAR then
            appear(me)
        elseif a == IDLE then
            idle(me, s)
        elseif a == GO then
            go(me)
        elseif a == SLASH then
            slash(me, s)
        else
            error(string.format("ShadowMan's shadow: action %#x isn't SplitUp's (0x080B9494)", a))
        end
    end
    c:present()
end

-- 0x080B93AC: ShadowMan's sprite (08-08) with a shadow on the ground, in his palette 0 (0x0800D258: his actor
-- record's version times his palette step) and facing, on its panel on the ground with 1 HP; its collision (a
-- floating body of its side, hit by the other side's, hit modifier 3) on no panel, as SplitUp's has it (its Param3
-- 1); then its first tick. (Its NameID is 0x18E, a player's record with AI index 0x17, which nothing reads of an
-- object no hit reaches.)
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.hp, me.max_hp = HP, HP
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_3e, collision.hit_by_other_side, HIT_MOD)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

shadow.kind = new.kind {
    id = "shadowman/shadow",
    pool = "actor",
    state = { held = "object", orders = "u8", gone = "bool", wait = "u8", slash_damage = "u32" },
    -- 0x080B9384: its sprite steps unless the battle is dimmed (or paused).
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
            -- 0x080B986E: its sword goes with it.
            (me.state :: State).held = nil
            me.collision:free()
            me:free()
            return
        end
        if not battle.dimmed() then
            me:update_sprite_while_dimmed()
        end
    end,
}

-- 0x080B9880 with 0x080B98CC, as SplitUp calls them: `owner`'s shadow on (x, y), of his side and facing, ordered
-- to slash after `wait` ticks for the damage word `slash_damage`. (It spawns at the registers' values: the panel's
-- Y and the two damage words, which its init replaces.)
function shadow.send(owner: Object, x: number, y: number, wait: number, damage: number, slash_damage: number): Object?
    local o = battle.spawn(shadow.kind, Vec3.new(y, damage, slash_damage))
    if o then
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
        o.related1 = owner
        local s = o.state :: State
        s.wait = wait
        s.slash_damage = slash_damage
        s.orders = SLASH_ORDER
    end
    return o
end

return shadow
