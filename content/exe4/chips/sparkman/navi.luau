--!strict
-- SparkMan, the navi of EXE4's SparkMan chips (EXE4's actor object #0x10, 0x080B89C0: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x0E, 0x080E2FEC: lib/navi_chips' short course). He appears on his
-- user's panel and, 31 ticks later (gone if it isn't solid), charges up (11 ticks); then flashes: the screen flickers
-- white, the other side's navi is blinded where it stands (a hit of no damage, blindness for 480 ticks), and his sparks
-- (./spark) start zigzagging over the field from the panel ahead of him; 28 ticks into his flash a shock strikes the
-- panel ahead of him. Once his sparks are gone and his 20 ticks have run, he goes in a puff.

local collision = require("../../rules/collision")
local navi_chips = require("../../lib/navi_chips")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local spark = require("./spark")
local statuses = require("../../rules/status")

local sparkman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- His sparks out (+0x60: each counts itself).
    sparks: number,
}

local SPRITE = asset.sprite("sparkman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_CHARGE = asset.sound("spark-charge")
local SOUND_FLASH = asset.sound("spark-flash")
-- The puff he goes in (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local SOLID = 0x10
-- His sparks' element (elec), whatever his chip's.
local ELEC = 3
-- The flash's flicker (a white palette flash, effect #0x0A's variant 0, going on while dimmed).
local FLASH_TICKS = 0x14
-- His charge, his flash's ticks, and the shock's.
local CHARGE, FLASH, SHOCK = 10, 20, 28

local APPEAR, ATTACK = 0x0, 0x4

-- 0x080B8A80: 31 ticks; then, on a solid panel, he charges up (animation 3, its sound, 11 ticks); else he goes.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_lifecycle("destroy")
        return
    end
    me.anim = 3
    battle.play_sound(SOUND_CHARGE)
    me.timer = CHARGE
    me:set_action(ATTACK)
end

-- His flash: his sparks from the panel ahead of him, going up; the blinding hit on the other side's navi (row 0x2E's,
-- no damage, no spark, blindness for 480 ticks), resolving while dimmed; the screen's flicker; its sound.
local function flash(me: Object)
    -- (0x080D1272: the first runs while dimmed.)
    local first = spark.spawn(me, me.panel_x + me.facing, me.panel_y, ELEC, true, 0, me)
    if first then
        first.run_while_dimmed = true
    end
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.other_navi[me.alliance],
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.collision_2e,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
        status = statuses.blind_480,
    })
    if box then
        box.run_while_dimmed = true
    end
    battle.palette_flash(0, FLASH_TICKS, true, false)
    battle.play_sound(SOUND_FLASH)
end

-- `sub_80C53A6`'s: the shock on the panel ahead of him (single, the elec spark, an elec attack at navis 16 pixels up,
-- modifier 3), resolving while dimmed.
local function shock(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + me.facing,
        panel_y = me.panel_y,
        element = ELEC,
        z = 0x10_0000,
        region = regions.single,
        hit_effect = sparks.elec,
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

-- 0x080B8AC0: his charge (animation 3) runs out: animation 4, his 20 ticks and the shock's 28. In animation 4, at a
-- frame without bit 0x80, his flash (animation 5). Else: the shock as its ticks run out; once his sparks are gone his
-- ticks run, and with both run out he goes.
local function attack(me: Object)
    local s = me.state :: State
    if me.anim == 3 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        me.anim = 4
        me.timer = FLASH
        me.timer2 = SHOCK
        return
    end
    if me.anim == 4 and not bit32.btest(me.sprite.frame_flags, 0x80) then
        me.anim = 5
        flash(me)
        return
    end
    if me.timer2 ~= 0 then
        me.timer2 -= 1
        if me.timer2 == 0 then
            shock(me)
        end
    end
    if s.sparks ~= 0 then
        return
    end
    if me.timer ~= 0 then
        me.timer -= 1
        if me.timer ~= 0 then
            return
        end
    end
    if me.timer2 == 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS: { [number]: (Object) -> () } = { [APPEAR] = appear, [ATTACK] = attack }

-- 0x080B8A64: his action.
local function tick(me: Object)
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("SparkMan's action %#x reads past his table (0x080B8A78)", me.action))
    end
    f(me)
end

-- 0x080B8A04: his sprite (08-13) with his shadow, shown, in animation 9, his chip's palette, facing his side's way, on
-- his panel; his sound; 31 ticks to his charge; then his first tick.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 9, 9
    sprite:set_animation(9)
    sprite.palette = s.palette
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND_APPEAR)
    me.timer = 0x1E
    me:set_lifecycle("update")
    tick(me)
end

sparkman.kind = new.kind {
    id = "sparkman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", sparks = "u32" },
    -- 0x080B89C0: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080B89E4: a puff 16 pixels above him (effect row 0x13); the controller moves on; he is freed.
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080B8B94: SparkMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0, of which his init keeps the height.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(sparkman.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.controller = controller
    s.palette = palette
    return o
end

-- What each SparkMan chip states for its own controller: the controller's id (the original's effect object #0x0E for
-- all three: compat maps each to it) and his palette (0x080B8A30 by the chip's level: 0, the SP's 3, the DS's 6).
export type ChipSpec = { id: string, palette: number }

-- A SparkMan chip's controller: the short course (the user warps out, 11 ticks, SparkMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "SparkMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
