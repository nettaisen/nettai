--!strict
-- WindMan's tornado (attack object #0x6B, 0x080DAC54: EXE4's own) and the sand it drops (attack #0x6C, 0x080DAF54):
-- from the panel ahead of him it goes a row down (or, on the bottom row, at once on), two panels ahead, back up its
-- column to the field's edge, then back toward his side until it leaves the field or, seven panels on, four ticks
-- into the next. A hit on each panel it enters (its region set again there; a hit clears it), and its panel
-- highlighted. On a pitfall (SandRing's sand) it picks up the sand (palette 1, the panel to normal); from then on
-- each panel it enters gets a hit of sand 11 ticks later.

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local tornado = {}

type State = {
    -- Ticks on its panel (+0x60) and panels entered (+0x0C), and it carries sand (+0x0D).
    ticks: number,
    steps: number,
    sandy: boolean,
}

local SPRITE = asset.sprite("windman-tornado")
local SOUND = asset.sound("tornado")
local SPEED = 0x4_0000
local DOWN, ACROSS, UP, BACK = 0x0, 0x4, 0x8, 0xC

-- The sand's hit: 11 ticks after it lands (0x080DAF54), on its panel (`sub_80C53A6`: single, the plain spark, a
-- collision row 0x20 at navis on the floor, hit modifier 3), resolving while dimmed; then it is freed. (It has no
-- state: its timer counts from its first update.)
local sand_kind = new.kind {
    id = "windman/sand",
    pool = "attack",
    update = function(me: Object)
        if me.phase_init == 0 then
            me.timer = 10
            me.phase_init = 4
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        local box = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.plain,
            target = collision.hits_navis,
            self_type = collision.collision_20,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if box then
            box.run_while_dimmed = true
        end
        me:free()
    end,
}
tornado.sand_kind = sand_kind

-- 0x080DAF88: sand of the tornado's on its panel, of its side and element, with its damage word, running while
-- dimmed. (Its position is the spawner's registers, which it keeps: the row, the element and 0.)
local function drop_sand(me: Object)
    local o = battle.spawn(sand_kind, Vec3.new(me.panel_y, me.element, 0))
    if not o then
        return
    end
    o.element = me.element
    o.panel_x, o.panel_y = me.panel_x, me.panel_y
    o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    o.alliance = me.alliance
    o.related1 = me
    o.run_while_dimmed = true
end

-- 0x080DAF14: on a pitfall it picks up the sand (palette 1, the panel to normal); carrying it, it drops some on its
-- panel. (Off the field the game reads the BIOS's open bus for the panel's type, which is never a pitfall's 10.)
local function sand(me: Object)
    local s = me.state :: State
    if not s.sandy then
        local p = field.panel(me.panel_x, me.panel_y)
        if p == nil or p.kind ~= "pitfall" then
            return
        end
        s.sandy = true
        me.sprite.palette = 1
        field.set_type(me.panel_x, me.panel_y, "normal")
    end
    drop_sand(me)
end

-- `sub_800E6E8` (0x0800AE10): a move from `old` to `new` reached `target` (past it, or onto it).
local function reached(new: number, old: number, target: number): boolean
    if new > old then
        return target > old and target <= new
    end
    return target > new and target <= old
end

-- Its panel from its coordinates, its collision's with it, a tick more on it.
local function moved(me: Object)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local s = me.state :: State
    s.ticks += 1
end

local function sign(v: number): number
    return if v >= 0 then 1 else -1
end

-- Action 0 (0x080DAD70): toward the next row's center (down first); reaching it (or with no row there), its rows'
-- way turned, on across.
local function down(me: Object)
    local v = me.vel
    if me.phase_init == 0 then
        me.phase_init = 4
        local y = me.panel_y + sign(v.y)
        if not field.valid(me.panel_x, y) then
            me.vel = Vec3.new(v.x, -v.y, v.z)
            me:set_action(ACROSS)
            return
        end
        me.future_panel_x, me.future_panel_y = me.panel_x, y
    end
    local _, ty = field.center(me.future_panel_x, me.future_panel_y)
    local p = me.pos
    local y = int.i32(p.y + v.y)
    me.pos = Vec3.new(p.x, y, p.z)
    if reached(y, p.y, ty) then
        me.vel = Vec3.new(v.x, -v.y, v.z)
        me:set_action(ACROSS)
        return
    end
    moved(me)
end

-- Action 4 (0x080DADD6): two panels' centers ahead along its row (reaching one, at once on to the next); then (or at
-- the field's edge) onto its panel's center, its way back, on up.
local function across(me: Object)
    local v = me.vel
    local function turn()
        me:set_coordinates_from_panel()
        me.vel = Vec3.new(-v.x, v.y, v.z)
        me:set_action(UP)
    end
    local count = me.phase_init == 0
    if count then
        me.timer2 = 2
        me.phase_init = 4
    end
    while true do
        if count then
            me.timer2 = int.u16(me.timer2 - 1)
            if int.i16(me.timer2) < 0 then
                return turn()
            end
            local x = me.panel_x + sign(v.x)
            me.future_panel_x, me.future_panel_y = x, me.panel_y
            if not field.valid(x, me.panel_y) then
                return turn()
            end
        end
        local tx = field.center(me.future_panel_x, me.future_panel_y)
        local p = me.pos
        local x = int.i32(p.x + v.x)
        me.pos = Vec3.new(x, p.y, p.z)
        if not reached(x, p.x, tx) then
            return moved(me)
        end
        count = true
    end
end

-- Action 8 (0x080DAE4A): along its column, each row's center reached at once on to the next, to the field's edge;
-- there onto its panel's center, on back.
local function up(me: Object)
    local v = me.vel
    local pick = me.phase_init == 0
    me.phase_init = 4
    while true do
        if pick then
            local y = me.panel_y + sign(v.y)
            if not field.valid(me.panel_x, y) then
                me:set_coordinates_from_panel()
                me:set_action(BACK)
                return
            end
            me.future_panel_x, me.future_panel_y = me.panel_x, y
        end
        local _, ty = field.center(me.future_panel_x, me.future_panel_y)
        local p = me.pos
        local y = int.i32(p.y + v.y)
        me.pos = Vec3.new(p.x, y, p.z)
        if not reached(y, p.y, ty) then
            return moved(me)
        end
        pick = true
    end
end

-- Action 0xC (0x080DAEAE): back along its row until off the screen (its X past -130 to 130 pixels) or, seven panels
-- entered, four ticks into its panel; then its region off, gone.
local function back(me: Object)
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    me.pos = Vec3.new(x, p.y, p.z)
    local s = me.state :: State
    local px = x // 0x1_0000
    if px + 130 < 0 or px + 130 > 260 or (s.ticks == 4 and s.steps >= 7) then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    moved(me)
end

local ACTIONS: { [number]: (Object) -> () } = { [DOWN] = down, [ACROSS] = across, [UP] = up, [BACK] = back }

-- 0x080DACFC: its hits resolve and spark (a hit takes its region); its action; on a new panel its region again, the
-- sand, its count; its panel highlighted.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local x, y = me.panel_x, me.panel_y
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("WindMan's tornado's action %#x reads past its table (0x080DAD60)", me.action))
    end
    f(me)
    if me.panel_x ~= x or me.panel_y ~= y then
        c:set_region(regions.single)
        sand(me)
        local s = me.state :: State
        s.ticks = 0
        s.steps = bit32.band(s.steps + 1, 0xFF)
    end
    field.highlight(me.panel_x, me.panel_y)
    c:present()
end

-- 0x080DAC78: not once the battle is over. Shown; its sprite (10-2F, no shadow) a frame on, facing its side's way,
-- on its panel on the floor; ahead and down 4 pixels a tick; the sand of its panel; a collision row 0x20 at navis
-- (modifier 3); its sound; then its first tick.
local function init(me: Object)
    if battle.over() then
        me:free()
        return
    end
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.timer = 0
    local s = me.state :: State
    s.ticks = 0
    me.vel = Vec3.new(panels.side_direction(me.alliance) * SPEED, SPEED, me.vel.z)
    sand(me)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_20, collision.hits_navis, 3)
    me.collision:present()
    me:set_lifecycle("update")
    battle.play_sound(SOUND)
    tick(me)
end

tornado.kind = new.kind {
    id = "windman/tornado",
    pool = "attack",
    state = { ticks = "u32", steps = "u8", sandy = "bool" },
    -- 0x080DAC54: by its state, then its sprite.
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

-- 0x080DAEEC: a tornado on (x, y) of WindMan's side, his, with `element` and his damage word, running while dimmed.
-- (Its position is the spawner's registers: the row, the element and 0.)
function tornado.spawn(windman: Object, x: number, y: number, element: number): Object?
    local o = battle.spawn(tornado.kind, Vec3.new(y, element, 0))
    if not o then
        return nil
    end
    o.element = element
    o.panel_x, o.panel_y = x, y
    o:set_damage_word(bit32.bor(windman.damage, bit32.lshift(windman.stamina, 16)))
    o.alliance = windman.alliance
    o.related1 = windman
    o.run_while_dimmed = true
    return o
end

return tornado
