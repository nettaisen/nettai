--!strict
-- The magnet field (attack object 0x75: EXE4's MagBolts', 0x080DBCA0; EXE5's MagnetSoul's, 0x080D4B60, the same code but
-- for its spawn and the paralysis it carries; EXE6's 0x75 is AirRaid's plane): an unseen attack on the panel at an
-- offset from its owner, following him, for 30 ticks: an attack against navis (hit modifier 3, its element's spark),
-- which paralyzes with no hit modifier when its parameter says. It goes once it hits something, when its owner's
-- first related object is cleared, or when the battle ends.
--
-- Each game makes its own (`magnet.make`): the paralysis it carries (EXE5's status byte 0x10, EXE4's 0x12), and
-- whether its spawn stores it in its owner's first related object (EXE5's, 0x080D4C32: MagnetSoul holds the field
-- itself) or only watches it (EXE4's, 0x080DBD72: the MagBolts' magnet is held there: `watches`).

local slot = require("../slot")

local magnet = {}

type State = {
    -- Param1: it paralyzes.
    paralyzes: boolean,
    -- +0x40 and +0x44: its offset from its owner's panel.
    dx: number,
    dy: number,
}

local HIT_MOD = 3
local LIFETIME = 30

-- Gone: no region, and freed at the next update; its time up, from its first action and phase (the game's word
-- store), else keeping them (its byte store: EXE5's 0x080D4BF0, EXE4's 0x080DBD30).
local function leave(me: Object, time_up: boolean)
    me.collision:set_region(nil)
    if time_up then
        me:set_lifecycle("destroy")
    else
        me.lifecycle = "destroy"
    end
end

-- EXE5's 0x080D4BF4, EXE4's 0x080DBD34: 30 ticks on its owner's panel and offset.
local function follow(me: Object)
    if me.phase_init == 0 then
        me.timer = LIFETIME
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        leave(me, true)
        return
    end
    local s = me.state :: State
    local owner = me.related1 :: Object
    me.panel_x, me.panel_y = owner.panel_x + s.dx, owner.panel_y + s.dy
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- EXE5's 0x080D4BBE, EXE4's 0x080DBCFE: its hits resolve and spark; while the battle runs, nothing was hit and its
-- owner's first related object holds something, it follows.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local owner = me.related1 :: Object
    if battle.over() or c.hit_flags ~= 0 or not slot.occupied(owner, "related") then
        leave(me, false)
        return
    end
    follow(me)
    c:present()
end

-- A game's magnet field: its spawn, the field `owner` holds (his first related object) or watches (`look.watches`),
-- at (dx, dy) from his panel, of `element` and damage word `damage`, paralyzing or not. (Its spawn position is the
-- spawner's registers, and it has no panel until its first update follows its owner.)
function magnet.make(look: MagnetLook): MagnetSpawn
    -- EXE5's 0x080D4B80, EXE4's 0x080DBCC0: on its panel, an attack against navis with its element's spark; then its
    -- first update.
    local function init(me: Object)
        me:set_coordinates_from_panel()
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.attack, look.hits, HIT_MOD)
        local c = me.collision
        c:set_hit_effect(look.spark_of(me.element))
        if (me.state :: State).paralyzes then
            c.status_base = look.paralysis
            c.hit_mod_base = 0
        end
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { paralyzes = "bool", dx = "i32", dy = "i32" },
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

    return function(owner: Object, dx: number, dy: number, element: number, paralyzes: boolean, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(dy, element, 0))
        if o then
            local s = o.state :: State
            s.paralyzes, s.dx, s.dy = paralyzes, dx, dy
            o.element = element
            o:set_damage_word(damage)
            o.alliance = owner.alliance
            o.related1 = owner
            if not look.watches then
                owner.related1 = o
            end
        end
        return o
    end
end

return magnet
