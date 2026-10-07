--!strict
-- ThunMan's thunderbolt (attack object #0x71, 0x080DB540: EXE4's own). On its panel, an elec body hitting navis (row
-- 10's, its spark the elec one), its region off: it marks its panel, highlighting it 4 of every 8 ticks for 32 ticks;
-- then it strikes with its sound (its region the panel, gone after a hit; a flash on a solid panel), breaking the panel
-- on its first tick (`object_breakPanel_dup1`), highlighted for 20 ticks, and goes. As the battle ends it goes at once.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local thunder = {}

export type State = {
    -- It breaks its panel as it strikes (Param1; ThunMan's always do), and it has (Param4).
    breaks: boolean,
    struck: boolean,
}

local SPRITE = asset.sprite("lightning")
local SOUND = asset.sound("lightning")
-- Its flash on a solid panel (effect row 0x47).
local FLASH = new.effect { sprite = asset.sprite("strike-flash"), anim = 0, palette = 0 }
local ELEC = 3

local MARK, STRIKE = 0x0, 0x4

-- 0x080DB620: 4 rounds of 8 ticks, the panel highlighted in each round's first 4 (but its first); then the strike.
local function mark(me: Object)
    if me.phase == 0 then
        me.timer = 4
        me.phase = 1
        me.timer2 = 8
        return
    end
    local t2 = me.timer2 - 1
    me.timer2 = int.u16(t2)
    if t2 == 0 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left > 0 then
            me.timer2 = 8
            return
        end
        me.action, me.phase = STRIKE, 0
        return
    end
    if t2 >= 4 then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- 0x080DB65E: shown, its sound, its region the panel, a flash on a solid panel; then its panel broken once,
-- highlighted, 20 ticks.
local function strike(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.timer = 0x14
        me.visible = true
        battle.play_sound(SOUND)
        me.collision:set_region(regions.single)
        if field.solid(me.panel_x, me.panel_y) then
            battle.effect(me.pos, FLASH)
        end
        me.phase = 1
        return
    end
    if not s.struck then
        if s.breaks then
            field.shatter(me.panel_x, me.panel_y)
        end
        s.struck = true
    end
    field.highlight(me.panel_x, me.panel_y)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080DB5CE: its hits resolve and spark; as the battle ends it goes; else, once it hit, its region off; its action;
-- presented.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
    else
        if bit32.btest(c.hit_flags, 0x3F80_0000) then
            c:set_region(nil)
        end
        local a = me.action
        if a == MARK then
            mark(me)
        elseif a == STRIKE then
            strike(me)
        else
            error(string.format("ThunMan's thunderbolt's action %#x reads past its table (0x080DB618)", a))
        end
        if me.lifecycle == "destroy" then
            c:set_region(nil)
            me:set_lifecycle("destroy")
        end
    end
    c:present()
end

-- 0x080DB564: its sheet (14-14), turned at random (one RNG draw), on its panel on the ground, elec; its collision
-- (row 10, hitting navis, modifier 3, the elec spark), its region off.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.band(battle.rng_positive(), 1))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.element = ELEC
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 3)
    me:update_collision_panels()
    me.collision:set_hit_effect(sparks.elec)
    ;(me.state :: State).struck = false
    me.collision:set_region(nil)
    me:set_lifecycle("update")
end

thunder.kind = new.kind {
    id = "thunman/thunder",
    pool = "attack",
    state = { breaks = "bool", struck = "bool" },
    -- 0x080DB540: by its state, then its sprite (`object_updateSprite`).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x08010560: its collision, then itself, freed.
            me.collision:free()
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DB6CE: a thunderbolt of `owner`'s side on (x, y) with the damage word `damage`, breaking its panel with
-- `breaks` (then running while dimmed too). (Its position is the spawner's registers, which its init overwrites.)
function thunder.spawn(owner: Object, x: number, y: number, damage: number, breaks: boolean): Object?
    local o = battle.spawn(thunder.kind, Vec3.zero)
    if o then
        o:set_damage_word(damage)
        ;(o.state :: State).breaks = breaks
        o.alliance = owner.alliance
        o.panel_x, o.panel_y = x, y
        if breaks then
            o.run_while_dimmed = true
        end
    end
    return o
end

return thunder
