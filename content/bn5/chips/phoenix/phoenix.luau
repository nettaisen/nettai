--!strict
-- The phoenix, Phoenix's (BN5's actor object #0x27, 0x080BFA70; BN5's own), brought by its controller
-- (./controllers). It appears at the back corner of its user's side (column 1 or 6, row 3) and, as it comes, picks
-- 18 target panels: 12 of the middle columns' (3 and 4) and 6 of columns 2 and 5, drawn at random and shuffled.
-- After a hit of no damage over the whole field it fades in, waits 30 ticks, then drops 18 fireballs (./fireball),
-- 8 ticks apart, one on each target, from over its own column (no more once the battle is over). 15 ticks on it
-- fades out; 60 ticks later its side's combatants heal three tenths of their HP; 10 more, and the controller moves
-- on.
--
-- Not reached: the palette flash it would show with each fireball when bit 19 of its update's r7 is set
-- (0x080BFBC8), which holds its list node's address, an EWRAM address under 0x02040000.

local controllers = require("./controllers")
local fireball = require("./fireball")
local collision = require("../../rules/collision")
local panels = require("@bn6/lib/panels")
local regions = require("@bn6/lib/regions")

local phoenix = {}

export type State = {
    -- The controller waiting on it (+0x54: the flag it was given).
    controller: Object?,
    -- Its targets, x | y << 4 (+0x60).
    targets: { number },
}

local SPRITE = asset.sprite("bn5:phoenix")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_HEAL = asset.sound("bn5:heal")
-- The healing's sparkle (the effect table's row 6).
local HEALED = define.effect { sprite = asset.sprite("bn5:heal"), anim = 0, palette = 0 }
-- The targets' columns (0x080BFD80, 0x080BFD86): the middle, and beside it.
local MIDDLE = { 0x13, 0x23, 0x33, 0x14, 0x24, 0x34 }
local BESIDE = { 0x12, 0x22, 0x32, 0x15, 0x25, 0x35 }
local TARGETS = 0x12
local FADE = 0x10
-- Where the fireballs start: over the phoenix's column at row 2, 50 pixels up.
local DROP_ROW, DROP_Z = 2, 0x32_0000

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- 0x080BFD3A: 12 targets of the middle columns, 6 beside them, each drawn (RNG % 6); then shuffled (18 swaps).
local function pick_targets(me: Object)
    local list: { number } = {}
    for _ = 1, 12 do
        table.insert(list, MIDDLE[battle.rng_positive() % 6 + 1])
    end
    for _ = 13, TARGETS do
        table.insert(list, BESIDE[battle.rng_positive() % 6 + 1])
    end
    panels.shuffle(list, TARGETS, TARGETS)
    local s = me.state :: State
    for i, p in list do
        s.targets[i] = p
    end
end

-- `ticks` ticks from the phase's first (counting down; on at 0), then `next`.
local function count_down(me: Object, ticks: number, next: () -> ())
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        next()
    end
end

-- Action 0 (0x080BFB0C): a hit of no damage over the whole field from panel (1, 1), running while dimmed (region
-- 0x80, no hit effect, target 5, self 0x27, hit modifier 0); then the fade-in.
local function field_hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = 1,
        panel_y = 1,
        element = 0,
        z = 0,
        region = regions.field,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.collision_27,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
    })
    if box then
        box.run_while_dimmed = true
    end
    me:set_action(4)
end

-- Action 4 (0x080BFB2A): shown, its sound; alpha 1 to 15 a tick; then solid.
local function fade_in(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.timer = 0
        battle.play_sound(SOUND_APPEAR)
        me.phase_init = 4
    end
    local t = int.u16(me.timer + 1)
    me.timer = t
    if t >= FADE then
        me.sprite.alpha = nil
        me:set_action(8)
        return
    end
    me.sprite.alpha = t
end

-- Action 8 (0x080BFB68): 30 ticks; then the fireballs from the first.
local function ready(me: Object)
    count_down(me, 0x1E, function()
        me.timer, me.timer2 = 0, 0
        me:set_action(0xC)
    end)
end

-- Action 0xC (0x080BFB92): a fireball every 8 ticks (the first at once) on the next target; after the 18th, or once
-- the battle is over, action 0x14.
local function drop(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.timer = 8
    if battle.over() then
        me:set_action(0x14)
        return
    end
    local target = (me.state :: State).targets[me.timer2 + 1]
    local x, y = field.center(me.panel_x, DROP_ROW)
    fireball.spawn(me, Vec3.new(x, y, DROP_Z), target, damage_word(me))
    me.timer2 += 1
    if me.timer2 >= TARGETS then
        me:set_action(0x14)
    end
end

-- 0x080BFCEA: each of its side's combatants heals three tenths of its maximum HP (rounded up), with the sparkle on
-- its panel and the sound.
local function heal_side(me: Object)
    for _, o in battle.alive_actors(me.alliance) do
        local x, y = field.center(o.panel_x, o.panel_y)
        battle.effect(Vec3.new(x, y, 0), HEALED)
        o.hp = math.min(o.hp + int.tdiv(o.max_hp * 3 + 9, 10), o.max_hp)
        battle.play_sound(SOUND_HEAL)
    end
end

-- Action 0x10 (0x080BFBF8): 30 ticks; the healing; action 0x20.
local function heal(me: Object)
    count_down(me, 0x1E, function()
        heal_side(me)
        me:set_action(0x20)
    end)
end

-- Action 0x14 (0x080BFC1E): 15 ticks.
local function linger(me: Object)
    count_down(me, 0xF, function()
        me:set_action(0x18)
    end)
end

-- Action 0x18 (0x080BFC40): alpha 15 down to 0 a tick; then hidden.
local function fade_out(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.timer = FADE
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        me.sprite.alpha = left
        return
    end
    me.visible = false
    me.sprite.alpha = nil
    me:set_action(0x1C)
end

-- Action 0x1C (0x080BFC7E): 30 ticks; then action 0x10.
local function gone(me: Object)
    count_down(me, 0x1E, function()
        me:set_action(0x10)
    end)
end

-- Action 0x20 (0x080BFCA0): 10 ticks; the controller moves on, and it goes.
local function leave(me: Object)
    count_down(me, 0xA, function()
        local c = (me.state :: State).controller
        if c then
            controllers.done(c)
        end
        me:set_lifecycle("destroy")
    end)
end

local ACTIONS = {
    [0] = field_hit,
    [4] = fade_in,
    [8] = ready,
    [0xC] = drop,
    [0x10] = heal,
    [0x14] = linger,
    [0x18] = fade_out,
    [0x1C] = gone,
    [0x20] = leave,
}

-- 0x080BFAD4.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("Phoenix's phoenix: action %#x reads past its table (0x080BFAE8)", me.action))
    end
    action(me)
end

-- 0x080BFA94: its sprite (animation 0, no shadow), facing its side's way, at the back corner of its side (column 1
-- or 6, row 3), 3 pixels down the field and 3 up; its targets. Hidden until its fade-in.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    local facing = bit32.bxor(me.alliance, me.flip)
    sprite:set_flip(facing)
    me.panel_x, me.panel_y = facing * 5 + 1, 3
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y + 0x3_0000, 0x3_0000)
    pick_targets(me)
    me:set_lifecycle("update")
end

phoenix.kind = define.kind {
    id = "bn5:phoenix/phoenix",
    pool = "actor",
    state = { controller = "object", targets = "u8[18]" },
    -- 0x080BFA70.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BFCC4: the phoenix for `user`, on `owner`'s panel (its controller's), of the user's side and facing, with
-- the damage word. (Its position is the spawner's registers, which its init overwrites.)
function phoenix.spawn(owner: Object, user: Object, damage: number): Object?
    local o = battle.spawn(phoenix.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
        o.element = owner.element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
        local s = o.state :: State
        s.controller = owner
    end
    return o
end

return phoenix
