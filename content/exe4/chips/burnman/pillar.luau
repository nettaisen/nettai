--!strict
-- BurnMan's pillars (attack object #0x3A, 0x080D46DC: EXE4's own; EXE5's and EXE6's #0x3A is the napalm bomb), the two
-- he raises two columns ahead of him, a row above and a row below his: each is his sprite (its animations 9 to 11, the
-- lower pillar's 12 to 14), a body hitting navis on its panel; after 20 ticks it rises (30 ticks, the lower 40), then
-- flames down (up) its column, a tick's hit on the two panels toward his row (./navi's lingering hits) and a roar, its
-- panels highlighted until it goes (78 ticks, the lower 68). A hit on it ends it in an impact.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local pillar = {}

type State = {
    -- The lower one (Param2), and BurnMan's palette.
    lower: boolean,
    palette: number,
}

local SPRITE = asset.sprite("burnman")
local SOUND_FLAME = asset.sound("roar")
local SOUND_HIT = asset.sound("hit-bomb-1")
-- The impact a hit ends it in (effect row 0x0F), 8 pixels up.
local IMPACT = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0 }
local FIRE = 1
-- Its X from its panel's center: the upper one's 8.5 pixels right, the lower one's left (0x080D47A0).
local SHIFT = 0x8_8000

local WAIT, FLAME = 0x0, 0x4

-- Its animation of `first` (the upper one's; the lower one's three on).
local function anim(me: Object, first: number): number
    return first + if (me.state :: State).lower then 3 else 0
end

-- Toward BurnMan's row: the upper one down, the lower one up.
local function toward(me: Object): number
    return if (me.state :: State).lower then -1 else 1
end

-- Its end: its region off, gone.
local function ends(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- Action 4, phase 0 (0x080D4830): it rises (30 ticks, the lower 40); then its flame: the lingering hits on the two
-- panels toward his row, a tick each; animation 11 (14), the roar; 78 ticks (68).
local function rise(me: Object, hits: (Object, number, number) -> ())
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim = anim(me, 10)
        me.timer = if s.lower then 40 else 30
    end
    me.timer = int.u16(me.timer - 1)
    if me.timer ~= 0 then
        return
    end
    local d = toward(me)
    hits(me, 0, d)
    hits(me, 0, 2 * d)
    me.anim = anim(me, 11)
    battle.play_sound(SOUND_FLAME)
    me.timer = if s.lower then 68 else 78
    -- (A halfword store: the phase's entry flag too.)
    me.phase, me.phase_init = 4, 0
end

-- Action 4, phase 4 (0x080D488A): its two panels highlighted (regions 0x19 and 0x18 from the panel toward his row)
-- until its ticks run out; then it goes.
local function burn(me: Object)
    local s = me.state :: State
    local region = if s.lower then regions.tall else regions.region_19
    local y = me.panel_y + toward(me)
    for _, o in region.panels or {} do
        field.highlight(me.panel_x + o[1] * me.facing, y + o[2])
    end
    me.timer = int.u16(me.timer - 1)
    if me.timer == 0 then
        ends(me)
    end
end

function pillar.make(hits: (owner: Object, dx: number, dy: number) -> ()): { kind: Kind, spawn: (Object, number, number, number, boolean) -> Object? }
    -- 0x080D47A8: its hits resolve and spark; the battle over, it goes; a hit on it ends it in an impact (8 pixels up)
    -- with a sound; else its action, and its body there again.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if battle.over() then
            return ends(me)
        end
        if c.hit_flags ~= 0 then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x8_0000), IMPACT)
            battle.play_sound(SOUND_HIT)
            return ends(me)
        end
        if me.action == WAIT then
            -- 0x080D4800: 20 ticks.
            local left = me.timer - 1
            me.timer = int.u16(left)
            if left <= 0 then
                me:set_action(FLAME)
            end
        elseif me.action == FLAME then
            if me.phase == 0 then
                rise(me, hits)
            elseif me.phase == 4 then
                burn(me)
            else
                error(string.format("BurnMan's pillar's phase %#x reads past its table (0x080D4828)", me.phase))
            end
        else
            error(string.format("BurnMan's pillar's action %#x reads past its table (0x080D47F8)", me.action))
        end
        c:present()
    end

    -- 0x080D4708: on its panel's center (gone at once off the field); his sprite in its animation 9 (12) with its shadow,
    -- his palette, facing its side's way; a body of row 4 hitting navis (modifier 3, the plain spark), its region its
    -- panel; shifted; 20 ticks.
    local function init(me: Object)
        local s = me.state :: State
        me:set_coordinates_from_panel()
        if not field.valid(me.panel_x, me.panel_y) then
            me:free()
            return
        end
        local sprite = me.sprite
        sprite:load(SPRITE)
        sprite.shadow = "ground"
        me.visible = true
        local a = anim(me, 9)
        me.anim, me.anim_loaded = a, a
        sprite:set_animation(a)
        sprite.palette = s.palette
        sprite:set_flip(me.alliance)
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(collision.attack, collision.hits_navis, 3)
        local c = me.collision
        c:set_region(regions.single)
        c:set_hit_effect(sparks.plain)
        c:present()
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + if s.lower then -SHIFT else SHIFT), p.y, p.z)
        me.timer = 20
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = "burnman/pillar",
        pool = "attack",
        state = { lower = "bool", palette = "u8" },
        -- 0x080D46DC: by its state, then its sprite.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                -- 0x080D4700: its collision, then itself, freed.
                me.collision:free()
                me:free()
                return
            end
            if me.active then
                me:update_sprite()
            end
        end,
    }

    -- 0x080D48BC: a pillar of BurnMan's on (x, y), fire, with his damage word, running while dimmed; the lower one
    -- (`lower`); his palette. (Its position is the spawner's registers.)
    local function spawn(burnman: Object, x: number, y: number, palette: number, lower: boolean): Object?
        local o = battle.spawn(kind, Vec3.new(y, FIRE, 0))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o.element = FIRE
        o:set_damage_word(bit32.bor(burnman.damage, bit32.lshift(burnman.stamina, 16)))
        o.alliance = burnman.alliance
        o.run_while_dimmed = true
        local s = o.state :: State
        s.lower, s.palette = lower, palette
        return o
    end

    return { kind = kind, spawn = spawn }
end

return pillar
