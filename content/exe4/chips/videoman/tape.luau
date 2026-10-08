--!strict
-- VideoMan's tape (attack object #0x5D, 0x080D90C8: EXE4's own), a reel over a square of four panels (the one it is
-- spawned on, the one ahead of it to the right and the two below), 20 pixels right, 46 down and 24 up from its panel's
-- center: it winds in (animation 16, its sound) and, every 15 ticks, three times, hits its four panels (a hit of no
-- element at navis, its height, modifier 3), highlighting them; then it unwinds (animation 17) and goes as that ends,
-- or at once when the battle is over.
--
-- (Its spawner's Param3 picks its course: VideoMan's 1. Actor #0x2F, EXE4's other VideoMan (0x080C2FC8), spawns it
-- with 0: a wait of 30 less 10 by its Param1 with its four panels highlighted, the tops' sound, and hits of modifier
-- 1 (0x080CD7E2); it waits on that actor, which no netbattle spawns.)

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local tape = {}

type State = {
    -- Its rounds of hits (Param2).
    rounds: number,
}

local SPRITE = asset.sprite("videoman")
local SOUND = asset.sound("tape")
-- Its rounds of hits, their period (15 >> Param1: VideoMan's 0), and its place off its panel's center.
local ROUNDS, PERIOD = 3, 15
local DX, DY, Z = 0x14_0000, 0x2E_0000, 0x18_0000
local WINDING, UNWINDING = 0x4, 0x8

-- Its four panels: its own, the one to its right, the one below that, the one below it.
local function square(me: Object): { { number } }
    local x, y = me.panel_x, me.panel_y
    return { { x, y }, { x + 1, y }, { x + 1, y + 1 }, { x, y + 1 } }
end

-- 0x080D926A: a hit on (x, y) (`sub_80C53A6`: single, the plain spark, an attack of its element at navis at its
-- height, hit modifier 3), resolving while dimmed.
local function hit(me: Object, x: number, y: number)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = me.pos.z,
        region = regions.single,
        hit_effect = sparks.plain,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- Action 4 (0x080D91E2): its period; then its hits and highlights (0x080D9290); after the third, it unwinds.
local function winding(me: Object)
    me.timer = int.u16(me.timer - 1)
    if me.timer ~= 0 then
        return
    end
    local panels = square(me)
    for _, p in panels do
        hit(me, p[1], p[2])
    end
    for _, p in panels do
        field.highlight(p[1], p[2])
    end
    local s = me.state :: State
    s.rounds += 1
    if s.rounds >= ROUNDS then
        me.anim = 17
        -- (The game sets its action with a byte store and writes its phase's halfword through a null pointer.)
        me.action = UNWINDING
    else
        me.timer = PERIOD
    end
end

-- Action 8 (0x080D923A): as its animation ends, it goes.
local function unwinding(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:set_lifecycle("destroy")
    end
end

-- 0x080D918C: once the battle is over it goes; else its action.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if me.action == WINDING then
        winding(me)
    elseif me.action == UNWINDING then
        unwinding(me)
    else
        error(string.format("VideoMan's tape's action %#x: Param3 0's course (actor #0x2F's), not VideoMan's", me.action))
    end
end

-- 0x080D90EC: on its panel's center (gone off the field), moved to its place; the sheet 08-17 (no shadow), shown, in
-- animation 15, palette 0, facing its side's way; then (Param3 1) winding in: animation 16, its sound, 15 ticks.
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + DX), int.i32(p.y + DY), Z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 15, 15
    sprite:set_animation(15)
    sprite.palette = 0
    sprite:set_flip(me.alliance)
    me:set_lifecycle("update")
    me.anim = 16
    battle.play_sound(SOUND)
    me.timer = PERIOD
    -- (The game sets its action with a byte store and writes its phase's halfword through a null pointer.)
    me.action = WINDING
end

tape.kind = new.kind {
    id = "videoman/tape",
    pool = "attack",
    state = { rounds = "u8" },
    -- 0x080D90C8: by its state, then its sprite.
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

-- 0x080D92BC: a tape on (x, y) of VideoMan's side, of no element, with his damage word, running while dimmed. (Its
-- position is the spawner's registers: the row, the element 0 and 0.)
function tape.spawn(videoman: Object, x: number, y: number): Object?
    local o = battle.spawn(tape.kind, Vec3.new(y, 0, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o:set_damage_word(bit32.bor(videoman.damage, bit32.lshift(videoman.stamina, 16)))
    o.alliance = videoman.alliance
    o.run_while_dimmed = true
    return o
end

return tape
