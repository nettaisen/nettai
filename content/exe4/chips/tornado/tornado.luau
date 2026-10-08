--!strict
-- EXE4's tornado (attack object #0x31, 0x080D2FDC: EXE4's own, where EXE5's and EXE6's take a panel's look): it spins
-- on its panel (a valid one) for its ticks, its region re-armed every 4 ticks (every 2 on SandRing's pitfall, type 10,
-- whose look it takes and which it turns normal as it ends), so it hits what stands there again and again: row 0x20,
-- hitting navis with hit modifier 1 and its element's spark, and a status of its own. Its sprite 0c-17 (drawn with its
-- shadow) in its palette, its sound 0x126; hidden and gone at its end or the battle's.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local tornado = {}

local SPRITE = asset.sprite("tornado")
local SOUND = asset.sound("tornado")
-- SandRing's pitfall: twice the re-arms, its own look, and normal after.
local PITFALL: PanelType, NORMAL: PanelType = "pitfall", "normal"
local PITFALL_PALETTE = 2
local REARM = 4
local HIT_MOD = 1

-- A tornado's look and rhythm: its palette, the status its hits give (none: none) and its ticks.
export type TornadoLook = { palette: number, status: Status?, ticks: number }

type State = {
    look: TornadoLook,
    -- How much of the re-arm count one tick uses (+0x60), and the count (+0x22).
    step: number,
    count: number,
}

-- Hidden, harmless, gone.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D30E6: its time; the region re-armed as the count runs out; at its end, a pitfall under it normal.
local function spin(me: Object, s: State)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t == 0 then
        local p = field.panel(me.panel_x, me.panel_y)
        if p and p.kind == PITFALL then
            field.set_type(me.panel_x, me.panel_y, NORMAL)
        end
        finish(me)
        return
    end
    local left = s.count - s.step
    s.count = int.u16(left)
    if left > 0 then
        return
    end
    s.count = REARM
    me.collision:set_region(regions.single)
end

-- 0x080D30AA: the hits against it resolve and spark; the battle's end hides it; a hit takes its region; its spin; its
-- sprite; present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        finish(me)
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    spin(me, me.state :: State)
    me:update_sprite()
    c:present()
end

-- 0x080D2FFC: on a valid panel, shown; its sprite (drawn with its shadow), turned to its side; on its panel, on the
-- ground; its re-arm (twice as quick on a pitfall, in the pitfall's palette); its collision; its sound; its first tick.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local s = me.state :: State
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:set_flip(me.alliance)
    s.step = 1
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    s.count = REARM
    local palette = s.look.palette
    local panel = field.panel(me.panel_x, me.panel_y)
    if panel and panel.kind == PITFALL then
        palette = PITFALL_PALETTE
        s.step *= 2
    end
    sprite.palette = palette
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_20, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.of_element(me.element))
    if s.look.status then
        c.status_base = s.look.status
    end
    c:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

local kind = new.kind {
    id = "tornado/tornado",
    pool = "attack",
    state = { look = "record:tornado-look", step = "u16", count = "u16" },
    -- 0x080D2FDC (its sprite in its tick).
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

function tornado.look(look: TornadoLook): TornadoLook
    return new.record("tornado-look", look)
end

-- 0x080D3130: a tornado of `user`'s on (x, y) of `look`, with the attack's element and damage word. (Its position is
-- the spawner's registers until its init places it: Y, the element and 0.)
function tornado.spawn(user: Object, x: number, y: number, element: number, damage: number, look: TornadoLook)
    local o = battle.spawn(kind, Vec3.new(y, element, 0))
    if o then
        o.element = element
        o.panel_x, o.panel_y = x, y
        o.timer = look.ticks;
        (o.state :: State).look = look
        o:set_damage_word(damage)
        o.alliance = user.alliance
    end
end

return tornado
