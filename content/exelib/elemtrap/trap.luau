--!strict
-- ElemTrap's trap, as EXE5 and EXE6 share it (EXE6's attack object #0x4D,
-- `sub_80CDF84`; EXE5's #0x4D, 0x080CF590): an
-- invisible collision over the whole field, even while the battle is
-- dimmed, that feels the other side's hits. The first hit of fire, aqua,
-- elec or wood springs it: once the battle isn't dimmed it marks the enemy
-- navi's panels with sparkles and hands over to a counterattack of that
-- element (./strike) in a dimming of its own; the trap is gone. It also
-- goes when its side's defensive chip changes (the linked record no longer
-- names it), at the end of the battle, and if it is hit in the tick it
-- appears. See docs/engine/chips.md §3.6.9.
--
-- Each game makes its kind with `trap.make_kind(id, look)` (what it
-- watches, collides as, sparkles and strikes with) and sets it with
-- `trap.spawn` (the trap chips' controller, @exelib/traps/controller: EXE6's
-- ElemTrap, EXE5's AntiFire to AntiWood).

local panels = require("../panels")
local regions = require("../regions")
local strike = require("./strike")

export type State = {
    -- The chip and the Atk+ / cross bonus (object +0x30, +0x32), passed
    -- to the counterattack, whose telop names them.
    chip: Chip?,
    bonus: number,
    -- Hits spring it (not in the tick it appears).
    armed: boolean,
    -- The element that sprang it (the original's first parameter); in a
    -- trap that watches one element, that element from the start.
    sprung_by: number,
}

local trap = {}

-- Its collision (`object_setupCollisionData`: it is nothing, and reaches
-- the other side's attacks and objects; hit modifier 0) covers the whole
-- field.
local HIT_MOD = 0

-- What springs it: damage in these elements (fire to wood; the first one
-- that has some).
local FIRST_ELEMENT, LAST_ELEMENT = 1, 4

-- Its actions (`off_80CE02C`, byte offsets).
local WATCH, SPRING = 0, 4


-- `sub_80CE0A4`: sparkles on the enemy navi's panels, first to last.
local function sparkle(me: Object, look: ElemTrapLook)
    local marked = regions.other_navi[me.alliance].field :: PanelCondition
    local list = panels.filtered(marked.require, marked.forbid)
    for i = #list, 1, -1 do
        local p = list[i]
        local x, y = field.center(panels.x(p), panels.y(p))
        battle.effect(Vec3.new(x, y + look.sparkle_dy, look.sparkle_z), look.sparkle, battle.local_side())
        battle.play_sound(look.sparkle_sound)
    end
end

-- `sub_80CE05E`: sprung. Hit as it appeared, it just goes; else, once the
-- battle isn't dimmed, the sparkles and the counterattack (its user's
-- side's dimming, which the other side can't cut in on), and it goes.
local function spring(me: Object, look: ElemTrapLook)
    local s = me.state :: State
    if s.armed then
        if battle.dimmed() then
            return
        end
        sparkle(me, look)
        local user = me.related1
        if not user then
            error("ElemTrap's trap has no user (sub_80E360E reads through a null pointer)")
        end
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local counter = strike.spawn(look.strike_kind, user, s.sprung_by, damage, s.bonus)
        dimming.start(user.alliance, true, counter, user, { chip = s.chip, bonus = s.bonus })
    end
    battle.clear_linked(me.alliance)
    me:set_lifecycle("destroy")
end

-- `sub_80CE034`: the first of fire, aqua, elec and wood with damage this
-- tick springs it (in a trap that watches one element, that element's).
local function watch(me: Object, look: ElemTrapLook)
    local s = me.state :: State
    local first, last = FIRST_ELEMENT, LAST_ELEMENT
    if look.watches == "one" then
        first, last = s.sprung_by, s.sprung_by
    end
    for e = first, last do
        if me.collision:element_damage(e) ~= 0 then
            s.sprung_by = e
            me.collision:set_region(nil)
            me.action, me.phase, me.phase_init = SPRING, 0, 0
            spring(me, look)
            return
        end
    end
end

-- The collision's panels and the object go (`object_clearCollisionRegion`,
-- `object_freeCollisionData`, `object_freeMemory`).
local function vanish(me: Object)
    me.collision:set_region(nil)
    me.collision:free()
    me:free()
end

-- `sub_80CDFEE`: resolve this tick's hits; unless its record went or the
-- battle is over, watch or spring, then register again. (The original's
-- `sub_802CEA6`, clearing a side's record, tells the record's object to
-- end through its second parameter; here the trap sees that its side's
-- record no longer names it.)
local function tick(me: Object, look: ElemTrapLook)
    me.collision:remove()
    if battle.linked(me.alliance).object ~= me then
        vanish(me)
        return
    end
    if battle.over() then
        battle.clear_linked(me.alliance)
        vanish(me)
        return
    end
    local a = me.action
    if a == WATCH then
        watch(me, look)
    elseif a == SPRING then
        spring(me, look)
    else
        error(string.format("ElemTrap's trap action %#x reads past its table", a))
    end
    me.collision:present()
end

-- `sub_80CDFA4`: its collision over the whole field, hit while dimmed; one
-- update unarmed, then armed.
local function init(me: Object, look: ElemTrapLook)
    me.panel_x, me.panel_y = 1, 1
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(look.self_type, look.target, HIT_MOD)
    me.collision:set_region(regions.field)
    me:set_status("hit_while_dimmed", true)
    me.collision:present()
    local s = me.state :: State
    s.armed = false
    me:set_lifecycle("update")
    tick(me, look)
    s.armed = true
end

-- A trap kind (`id`) of `look`.
function trap.make_kind(id: string, look: ElemTrapLook): Kind
    return new.kind {
        id = id,
        pool = "attack",
        state = { chip = "chip", bonus = "u16", armed = "bool", sprung_by = "u8" },
        -- `sub_80CDF84`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
            end
        end,
    }
end

-- `sub_80CE0EC`: a trap of `kind` for `user`, on `controller`'s side, with
-- the controller's damage word, its chip and the bonus; a trap that
-- watches one element watches `element`. (Its position is the spawner's
-- registers, which nothing reads.)
function trap.spawn(kind: Kind, controller: Object, user: Object, chip: Chip?, bonus: number, element: number?): Object?
    local o = battle.spawn(kind, Vec3.new(0, controller.element, 0))
    if not o then
        return nil
    end
    o.related1 = user
    o.element = 0
    o.damage, o.stamina = controller.damage, controller.stamina
    o.alliance = controller.alliance
    local s = o.state :: State
    s.chip, s.bonus = chip, bonus
    if element then
        s.sprung_by = element
    end
    o.run_while_dimmed = true
    return o
end

return trap
