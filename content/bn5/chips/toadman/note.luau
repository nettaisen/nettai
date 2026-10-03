--!strict
-- ToadMan's note (BN5's attack object #0x81, 0x080D66A0; BN5's own: BN6's #0x81 is TenguCross Beast's whirlwind, on
-- the same skeleton), an elec attack that paralyzes (status 0x10). Each note lives 10 ticks on its panel (gone at
-- once on a hit, off the field or at the battle's end) and 6 ticks in sends the next one panel ahead, a row nearer
-- its target's: a chain of notes stepping toward the target.

local collision = require("../../rules/collision")
local status = require("../../rules/status")

local note = {}

export type State = {
    -- What the chain heads for (+0x64) and the next note's row (+0x68).
    target: Object?,
    next_y: number,
    -- Its lifetime and its next note's ticks (Param2, Param1), and it runs while dimmed (Param0 1).
    life: number,
    delay: number,
    dimmed: boolean,
}

export type Chain = { life: number, delay: number, dimmed: boolean }

local SPRITE = asset.sprite("bn5:toad-note")
local ELEC = 3

-- Action 0 (0x080D6764): its timers.
local function start(me: Object)
    local s = me.state :: State
    me.timer = s.life
    me.timer2 = s.delay
    me:set_action(4)
end

-- Action 4 (0x080D6778): the next note when its delay runs out; gone when its life does.
local function live(me: Object)
    local s = me.state :: State
    local d = int.u16(me.timer2 - 1)
    me.timer2 = d
    if d == 0 then
        note.spawn(me, me.panel_x + me.vel.x, s.next_y, { life = s.life, delay = s.delay, dimmed = s.dimmed }, s.target)
    end
    local t = int.u16(me.timer - 1)
    me.timer = t
    if t == 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- 0x080D6714: its hits resolved; gone on a hit, off the field or at the battle's end (hidden); else its action.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 or not field.valid(me.panel_x, me.panel_y) or battle.over() then
        c:set_region(nil)
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == 0 then
        start(me)
    elseif a == 4 then
        live(me)
    else
        error(string.format("ToadMan's note: action %#x reads past its table (0x080D675C)", a))
    end
    c:present()
end

-- 0x080D66C4: its sprite (10-44, no shadow, animation 0), a thrown attack at navis (`sub_8011504`), on its panel,
-- on the ground, shown; its step (its facing) and the next note's row (one nearer its target's, if any); paralysis;
-- then its first update.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        error("ToadMan's note: no collision slot (0x080D66C4 writes through a null pointer)")
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.visible = true
    me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
    local y = me.panel_y
    local t = s.target
    if t and y ~= t.panel_y then
        y = if y < t.panel_y then y + 1 else y - 1
    end
    s.next_y = y
    local c = me.collision
    c.status_base = status.paralyze_90
    c:present()
    -- (0x08002DBE: its sprite unflipped after all.)
    sprite:set_flip(0)
    me:set_lifecycle("update")
    tick(me)
end

note.kind = define.kind {
    id = "bn5:toadman/note",
    pool = "attack",
    state = { target = "object", next_y = "u8", life = "u8", delay = "u8", dimmed = "bool" },
    -- 0x080D66A0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:load_or_step_sprite()
    end,
}

-- 0x080D67A6: a note of `owner`'s on (x, y), elec, with its damage word, its chain heading for `target`, of its side
-- and facing; running while dimmed for a chain that does.
function note.spawn(owner: Object, x: number, y: number, chain: Chain, target: Object?): Object?
    local o = battle.spawn(note.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = ELEC
    o:set_damage_word(bit32.bor(owner.damage, bit32.lshift(owner.stamina, 16)))
    o.alliance, o.flip = owner.alliance, owner.flip
    o.related1 = owner
    local s = o.state :: State
    s.target = target
    s.life, s.delay, s.dimmed = chain.life, chain.delay, chain.dimmed
    if chain.dimmed then
        o.run_while_dimmed = true
    end
    return o
end

return note
