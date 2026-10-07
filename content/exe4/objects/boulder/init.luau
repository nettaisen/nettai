--!strict
-- EXE4's boulder (attack object #0x6E, 0x080DB0EC): the field obstacle its netbattle stages start with (the actor
-- lists' entry type 3, 0x08006880), 500 HP, on its panel's side. EXE6's code (@exelib/boulder, `sub_80D2290`) but for
--
-- - its init (0x080DB10C) writes no NameID: it has no identity;
-- - its tick (0x080DB178) takes its hits never pushed (0x080139A0, EXE6's `sub_801AD6A`) and reacts by 0x08014058
--   (EXE6's `sub_801B4D4`, which its boulder doesn't call: destroyed, the HP as it is, by an obstacle's touch or a
--   breaking hit but no body's, and no removal request read: "destroys_sparing_bodies");
-- - its waiting (0x080DB1BC) keeps action 0 once the fight is on (EXE6's goes to its idle, 8) and, before it, hides
--   it where the battle's settings byte 6 has 0x20 (0x0801466A: no netbattle's, 0x1F);
-- - its action table (0x080DB1A0, 7 rows): row 1 (0x0801013A) also stops it running while paused, row 3
--   (0x08010276) does nothing, row 5 is the knock-back (0x080112C4, EXE6's `sub_8017CC0`), row 6 its idle, which
--   nothing starts but row 1;
-- - its leaving (0x080DB1EC) releases no tracking, and goes by its HP: left, a puff (effect 21) 12 pixels up; none,
--   two chunks of debris and a splash (effect 1) 16 up;
-- - its end (state 8) frees the collision and the object (0x08010560);
-- - EXE4's sheet (10-31), debris and effects (lib/rock), collision types (rows 14 and 15) and the damage it is thrown
--   for, 100 (its stage spawner's word, 0x080068A8).

local collision = require("../rules/collision")
local debris = require("@exelib/rock/debris")
local rock = require("../lib/rock")

local boulder = {}

local SPRITE = asset.sprite("boulder")
local HP = 500
-- What its timer starts at (nothing counts it down).
local TIMER = 0x1770
local HIT_MOD = 3
-- The palette of the debris it breaks into.
local DEBRIS_PALETTE = 0
-- The damage it does thrown (its spawner's damage word).
local DAMAGE = 100
-- A panel of side 1's.
local SIDE_1 = 0x20
-- The spawner's header flags (0x080DB27A, EXE6's `sub_80D2430`'s slip): the pause and dimming bits ORed into the byte
-- it reads through the panel's column, an address in the console's BIOS (what comes back is a byte of the opcode the
-- BIOS last fetched, 0xE3A02004, by the address's low two bits; @exelib/boulder).
local BIOS_WORD, RUNS_PAUSED_AND_DIMMED = 0xE3A02004, 0x14

-- Its actions in the framework's numbering (its table's 0, 2 and 6).
local WAIT, DESTROYED, STANDING = 0, 2, 8

-- 0x080DB1BC: once the fight is on, it no longer runs while the battle is paused, shows, and stays in this action.
local function wait(me: Object)
    if not battle.fighting() then
        return
    end
    me.run_while_paused = false
    me.visible = true
    me:set_action(obstacle.action_byte(me, WAIT))
end

-- 0x080DB1E8.
local function standing(_me: Object) end

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

-- 0x080DB1EC: leave the field and its stage slot: with HP left, a puff; broken, two chunks and a splash.
local function destroyed(me: Object)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.leave_stage(me)
    local p = me.pos
    if me.hp ~= 0 then
        battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), rock.effects.puff)
    else
        for _ = 1, 2 do
            debris.spawn(rock.debris, battle.jitter(0xF, p), DEBRIS_PALETTE)
        end
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), rock.effects.splash)
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- The boulder's action table (0x080DB1A0), by the framework's numbers.
local ACTIONS: { [number]: (Object) -> () } = {
    [WAIT] = wait,
    [1] = return_to_idle,
    [DESTROYED] = destroyed,
    [3] = function(_me: Object) end,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [STANDING] = standing,
}

-- 0x080DB178: hits, the reaction, its action; no hit spark and no lifetime. Its sprite steps while dimmed too.
local function tick(me: Object)
    me.visible = true
    obstacle.take_hits(me, "ignored")
    local a = obstacle.react(me, "destroys_sparing_bodies")
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("boulder action %d reads past its table", a))
        end
        action(me)
    end
    me.collision:present()
    me:update_sprite_while_dimmed()
end

-- 0x080DB10C: on its panel, a pixel back and a pixel down, with its collision on the panel at once. Its first update
-- is the next tick's.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "ground"
    me:set_coordinates_from_panel()
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y - 0x1_0000, -0x1_0000)
    me.hp, me.max_hp = HP, HP
    me.timer = TIMER
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.neutral_body, collision.everything, HIT_MOD)
    me.collision:present()
    me.lifecycle = "update"
end

boulder.kind = new.kind {
    id = "boulder",
    pool = "attack",
    -- 0x080DB0EC.
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
    -- 0x08006880: a stage's boulder (an actor list's entry type 3), on its panel's side, if one of the field's two
    -- stage slots is free; running while the battle is paused or dimmed, with the flags its spawner's slip leaves.
    -- (0x080DB25E; until its init places it, its position is the spawner's registers: Y and what nothing reads.)
    place = function(spec: PlaceSpec): Object?
        if not obstacle.stage_slot_free() then
            return nil
        end
        local x, y = spec.panel_x, spec.panel_y
        local o = battle.spawn(boulder.kind, Vec3.new(y, 0, 0))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(DAMAGE)
        o.alliance = if bit32.btest(field.flags(x, y), SIDE_1) then 1 else 0
        local read = bit32.band(bit32.rshift(BIOS_WORD, 8 * (x % 4)), 0xFF)
        o:set_header_flags(bit32.bor(read, RUNS_PAUSED_AND_DIMMED))
        obstacle.enter_stage(o)
        return o
    end,
}

return boulder
