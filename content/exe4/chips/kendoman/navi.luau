--!strict
-- KendoMan, the navi of EXE4's KendoMan chips (EXE4's actor object #0x43, 0x080C85E8: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x66, 0x080E9430: lib/navi_chips' short course). He appears on his
-- user's panel (no element of his own) and, 31 ticks later (gone if it isn't solid), dashes along his row to the panel
-- before the first enemy body in it (or the field's edge), leaving afterimages, and slashes the panel ahead; then
-- vanishes in a puff, reappears two panels on facing back, glides 8 ticks and cuts the column behind him; then, sword
-- out and a hit on his own panel that follows him, rushes back toward his side until he leaves solid ground, and goes.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local lingering_hit = require("../../objects/lingering_hit")
local link_chips = require("@exelib/link_chips")
local navi_chips = require("../../lib/navi_chips")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local kendoman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x48).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- Where his dash ends (+0x7C, +0x7D).
    target_x: number,
    target_y: number,
}

local SPRITE = asset.sprite("kendoman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_DASH = asset.sound("counter-strike")
local SOUND_VANISH = asset.sound("sword-swing")
local SOUND_DRAW = asset.sound("kendo-draw")
-- The puff he vanishes and goes in (effect row 0x13), and his cut (effect row 0x17: the sword slash, palette 2 on).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local CUT = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
-- His sword (attachment row 0x2D: his own sheet, palette 0), in its animation 8, animating while dimmed.
local SWORD = attachment.look { sprite = SPRITE, palette = 0 }
local SWORD_ANIM = 8
-- His hit while he rushes back (0x080C892E).
local RUSH_HITS = lingering_hit.make("kendoman/rush-hit")
local RUSH_TICKS = 100

-- An enemy body on a panel, by his side (0x080C8A20).
local ENEMY_BODY: { [number]: number } = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local SOLID = 0x10
-- What his moves stop at (0x080C1DC8), and what his landing panel must be free of (0x080C8814).
local BLOCKERS, BODIES = 0x0380_0000, 0x0F80_0000
-- His afterimages' color shader, and their lifetimes on his dash and his rush.
local AFTERIMAGE_SHADER = 0x814A
local DASH_SPEED = 0x6_AAAB
local RUSH_SPEED = 0xC_0000

local APPEAR, DASH, CUT_BACK, RUSH = 0x0, 0x4, 0x8, 0xC

-- 0x080C1DC8: the panel under him is solid with no blocker on it.
local function on_ground(me: Object): boolean
    local x, y = link_chips.panel_at(me.pos.x, me.pos.y)
    return field.valid(x, y) and field.check(x, y, SOLID, BLOCKERS)
end

-- 0x080C20D0: his afterimage as he stands (his sheet, his animation, his side's flip), lasting `ticks`, with its
-- shadow.
local function afterimage(me: Object, ticks: number)
    battle.afterimage(me, me.pos, {
        sprite = SPRITE,
        anim = me.anim,
        flip = me.alliance,
        lifetime = ticks,
        color_shader = AFTERIMAGE_SHADER,
        shadow = "hidden",
    })
end

-- A puff 16 pixels above him (effect row 0x13).
local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
end

-- Off the solid ground: a puff, and onto column 0 (where his update ends him).
local function fall_off(me: Object)
    puff(me)
    me.panel_x = 0
    me:set_coordinates_from_panel()
end

-- His sword goes (0x080CD708).
local function sheathe(me: Object)
    local sword = me.related1
    if sword and sword.active then
        sword:set_lifecycle("destroy")
    end
end

-- 0x080C89E4: along his row from his panel, the panel before the first with an enemy body, or the field's edge (7, or
-- 0 for side 1).
local function dash_target(me: Object): (number, number)
    local dir = me.facing
    local edge = 7 * bit32.rshift(dir + 1, 1)
    local x = me.panel_x
    while true do
        if bit32.btest(field.flags(x, me.panel_y), ENEMY_BODY[me.alliance]) then
            return x - dir, me.panel_y
        end
        x += dir
        if x == edge then
            return x, me.panel_y
        end
    end
end

-- Action 0 (0x080C86D0): 31 ticks; then, on a solid panel, his dash; else he goes.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_action(DASH)
    else
        me:set_lifecycle("destroy")
    end
end

-- Action 4 (0x080C86FA): toward his target in 6 ticks (animation 3, its sound), an afterimage each other tick; there,
-- the slash on the panel ahead (single, the plain spark, an attack of no element at navis 16 pixels up, modifier 1),
-- animation 4; then 10 of its ends.
local function dash(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim = 3
        s.target_x, s.target_y = dash_target(me)
        me.timer = 6
        me.vel = Vec3.new(int.i32((s.target_x - me.panel_x) * DASH_SPEED), me.vel.y, me.vel.z)
        battle.play_sound(SOUND_DASH)
    end
    if me.phase_init == 1 then
        if not bit32.btest(me.timer, 1) then
            afterimage(me, 6)
        end
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
        if not on_ground(me) then
            return fall_off(me)
        end
        me:set_panel_from_coordinates()
        me.timer = int.u16(me.timer - 1)
        if me.timer ~= 0 then
            return
        end
        local box = battle.hitbox(me, {
            panel_x = me.panel_x + me.facing,
            panel_y = me.panel_y,
            element = 0,
            z = 0x10_0000,
            region = regions.single,
            hit_effect = sparks.plain,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 1,
        })
        if box then
            box.run_while_dimmed = true
        end
        me.anim = 4
        me.timer = 10
        me.phase_init = 2
        return
    end
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me.timer = int.u16(me.timer - 1)
        if me.timer == 0 then
            me:set_action(CUT_BACK)
        end
    end
end

-- Action 8 (0x080C87C0): a puff, hidden, its sound; 3 ticks; then two panels on, if solid with no body there (else
-- onto column 0, where his update ends him), shown in animation 0 facing back; 8 ticks gliding on 4 pixels a tick;
-- then the cut on the column behind him (region 4, no spark, an attack of no element at navis, modifier 1) with its
-- slash (effect row 0x17, palette 2 on), animation 7, onto his panel's center; then 10 of its ends.
local function cut_back(me: Object)
    local dir = me.facing
    if me.phase_init == 0 then
        me.phase_init = 1
        puff(me)
        me.visible = false
        battle.play_sound(SOUND_VANISH)
        me.timer = 3
    end
    if me.phase_init == 1 then
        me.timer = int.u16(me.timer - 1)
        if me.timer ~= 0 then
            return
        end
        local x = me.panel_x + 2 * dir
        if not (field.valid(x, me.panel_y) and field.check(x, me.panel_y, SOLID, BODIES)) then
            me.panel_x = 0
            me:set_coordinates_from_panel()
            return
        end
        me.panel_x = x
        me:set_coordinates_from_panel()
        me.visible = true
        me.anim = 0
        me.sprite:set_flip(bit32.bxor(me.alliance, 1))
        me.timer = 8
        me.phase_init = 2
    end
    if me.phase_init == 2 then
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + dir * 0x4_0000), p.y, p.z)
        me.timer = int.u16(me.timer - 1)
        if me.timer ~= 0 then
            return
        end
        local x = me.panel_x - dir
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = me.panel_y,
            element = 0,
            z = 0,
            region = regions.wide,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 1,
        })
        if box then
            box.run_while_dimmed = true
        end
        local cx, cy = field.center(x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), CUT, bit32.bxor(me.alliance, 1), 2)
        me.anim = 7
        me:set_coordinates_from_panel()
        me.timer = 10
        me.phase_init = 3
        return
    end
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me.timer = int.u16(me.timer - 1)
        if me.timer == 0 then
            me:set_action(RUSH)
        end
    end
end

-- Action 0xC (0x080C88E4): animation 5, his sound; as it ends, animation 6, his sword out, his rush's hit on his
-- panel (single, no spark, an attack of his element at navis, modifier 1, 100 ticks, following him), back toward his
-- side 12 pixels a tick, an afterimage (of his side turned) each fourth tick; off the solid ground, a puff, hidden,
-- his sword away, onto column 0 (where his update ends him).
local function rush(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim = 5
        battle.play_sound(SOUND_DRAW)
        return
    end
    if me.phase_init == 1 then
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        me.anim = 6
        attachment.attach(me, SWORD, "related", SWORD_ANIM, true)
        local hit = RUSH_HITS.spawn(me, 0, 0, {
            region = regions.single,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.attack,
            hit_mod = 1,
            damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
            z = 0,
            ticks = RUSH_TICKS,
            element = me.element,
        })
        if hit then
            hit.run_while_dimmed = true
        end
        me.vel = Vec3.new(panels.side_direction(bit32.bxor(me.alliance, 1)) * RUSH_SPEED, me.vel.y, me.vel.z)
        me.timer = 3
        me.phase_init = 2
    end
    me.timer = int.u16(me.timer + 1)
    if bit32.band(me.timer, 3) == 0 then
        -- (Its side turned while it is made: its alliance and flip.)
        local side = me.alliance
        me.alliance = bit32.bxor(side, 1)
        afterimage(me, 12)
        me.alliance = side
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    if not on_ground(me) then
        fall_off(me)
        me.visible = false
        sheathe(me)
        return
    end
    me:set_panel_from_coordinates()
end

local ACTIONS: { [number]: (Object) -> () } = { [APPEAR] = appear, [DASH] = dash, [CUT_BACK] = cut_back, [RUSH] = rush }

-- 0x080C869C: his action; off the field, he goes.
local function tick(me: Object)
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("KendoMan's action %#x reads past his table (0x080C86C0)", me.action))
    end
    f(me)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
    end
end

-- 0x080C8638: his sprite (08-0D) with his shadow, shown, in animation 0, his chip's palette, facing his side's way, on
-- his panel, of no element; his sound; 30 ticks; then his first tick.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = s.palette
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    me.element = 0
    battle.play_sound(SOUND_APPEAR)
    me.timer = 30
    me:set_lifecycle("update")
    tick(me)
end

kendoman.kind = new.kind {
    id = "kendoman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", target_x = "u8", target_y = "u8" },
    -- 0x080C85E8: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C860C: a puff 16 pixels above him; the controller moves on; his sword away; he is freed. (The rush's
            -- hit he would end through his +0x60, which nothing fills: it keeps its 100 ticks.)
            puff(me)
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            sheathe(me)
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080C89C0: KendoMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(kendoman.kind, Vec3.new(controller.panel_y, controller.element, 0))
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

-- What each KendoMan chip states for its own controller: the controller's id (the original's effect object #0x66 for
-- all three: compat maps each to it) and his palette (0x080C8664 by the chip's level: 0, the SP's 3, the DS's 4).
export type ChipSpec = { id: string, palette: number }

-- A KendoMan chip's controller: the short course (the user warps out, 11 ticks, KendoMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "KendoMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
