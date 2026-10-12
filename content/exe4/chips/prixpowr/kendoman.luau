--!strict
-- PrixPowr's KendoMan (EXE4's actor object #0x55, 0x080CBE40: EXE4's own, not the KendoMan chips' #0x43), the last of
-- its three: the KendoMan chips' rush, across the whole field. He appears behind his side's edge in the user's row
-- (column 0, or 7 for side 1), sword out (animation 6, its sound, his sword an attachment), and rushes ahead 8 pixels
-- a tick, an afterimage each fourth tick, with a hit on his own panel that follows him (single, no spark, an attack
-- of his element at navis, hit modifier 3, 100 ticks): once it has ended on a body he makes another when he is a
-- panel on from where he saw it gone. He goes in a puff where the ground stops him (a panel inside the field that
-- isn't solid, or holds a blocker) or as he reaches the column past the far edge. He runs while dimmed.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local lingering_hit = require("../../objects/lingering_hit")
local link_chips = require("@exelib/link_chips")
local regions = require("@exelib/regions")

local kendoman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x48).
    controller: Object?,
    -- His rush's hit, which keeps itself here (his +0x60).
    hits: { Object? },
    -- Where he saw his hit gone (his +0x64, as the original packs a panel), and that he has (+0x68).
    gone_at: number,
    gone: boolean,
}

local SPRITE = asset.sprite("kendoman")
local SOUND_DRAW = asset.sound("kendo-draw")
-- The puff he goes in (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
-- His sword (attachment row 0x2D: his own sheet, palette 0), in its animation 8, animating while dimmed.
local SWORD = attachment.look { sprite = SPRITE, palette = 0 }
local SWORD_ANIM = 8
-- His rush's hit (0x080CC026).
local RUSH_HITS = lingering_hit.make("prixpowr/rush-hit")
local RUSH_TICKS = 100
-- His afterimages (0x080C20D0, the KendoMan chips'): their color shader and lifetime.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x814A, 12
local SPEED = 0x8_0000
local SOLID = 0x10
-- What stops him (0x080CBFD0).
local BLOCKERS = 0x0380_0000

-- 0x080CC026: a hit on his panel, which follows him and keeps itself in his `hits`.
local function hit(me: Object)
    local o = RUSH_HITS.spawn(me, 0, 0, {
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        hit_mod = 3,
        damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
        z = 0,
        ticks = RUSH_TICKS,
        element = me.element,
        holder = me,
        slot = 1,
    })
    if o then
        o.run_while_dimmed = true
    end
end

-- 0x080CBFF0: with no hit out, he notes the panel he is on; a panel on from there, another hit.
local function rearm(me: Object)
    local s = me.state :: State
    if s.hits[1] then
        return
    end
    local here = me.panel_x + me.panel_y * 0x10
    if not s.gone then
        s.gone_at, s.gone = here, true
    elseif here ~= s.gone_at then
        hit(me)
        s.gone = false
    end
end

-- 0x080CBF78: a puff 16 pixels above him; onto row 0 and hidden; his sword away (0x080CD708) and his hit cut to
-- its last tick (0x080DCD84); gone.
local function stop(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
    me.panel_y = 0
    me:set_coordinates_from_panel()
    me.visible = false
    local sword = me.related1
    if sword and sword.active then
        sword:set_lifecycle("destroy")
    end
    local out = (me.state :: State).hits[1]
    if out then
        out.timer = 1
    end
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080CBEF4), his only one: his sword out and his first hit; then his rush.
local function rush(me: Object)
    if me.phase_init ~= 1 then
        me.phase_init = 1
        me.anim = 6
        battle.play_sound(SOUND_DRAW)
        attachment.attach(me, SWORD, "related", SWORD_ANIM, true)
        hit(me)
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        me.timer = 5
        return
    end
    local t = me.timer + 1
    if t >= 4 then
        -- 0x080C20D0: his afterimage as he is, with its shadow hidden.
        battle.afterimage(me, me.pos, {
            sprite = SPRITE,
            anim = me.anim,
            flip = me.alliance,
            lifetime = AFTERIMAGE_TICKS,
            color_shader = AFTERIMAGE_SHADER,
            shadow = "hidden",
        })
        t = 0
    end
    me.timer = t
    rearm(me)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    local x = me.panel_x
    if x >= 1 and x <= 6 then
        -- 0x080CBFD0: the panel under him must be solid with no blocker on it.
        local px, py = link_chips.panel_at(me.pos.x, me.pos.y)
        if not field.check(px, py, SOLID, BLOCKERS) then
            return stop(me)
        end
    end
    if x == bit32.bxor(me.alliance, 1) * 7 then
        stop(me)
    end
end

-- 0x080CBE7E: his sprite (08-0D) with his shadow, shown, in animation 0, palette 0 (the chip's: 0x080CBEAC by its
-- first parameter, as the KendoMan chips' are), facing his side's way, on his panel; then his first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local s = me.state :: State
    s.gone_at, s.gone = 0, false
    me:set_lifecycle("update")
    rush(me)
end

kendoman.kind = new.kind {
    id = "prixpowr/kendoman",
    pool = "actor",
    state = { controller = "object", hits = "object[1]", gone_at = "u8", gone = "bool" } :: StateSchema,
    -- 0x080CBE40: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            if me.action ~= 0 then
                error(string.format("PrixPowr's KendoMan's action %#x reads past his table (0x080CBEF0)", me.action))
            end
            rush(me)
        else
            -- 0x080CBE64: the controller moves on; his sword away and his hit cut to its last tick; he is freed.
            local s = me.state :: State
            if s.controller then
                navi_chip.navi_left(s.controller)
            end
            local sword = me.related1
            if sword and sword.active then
                sword:set_lifecycle("destroy")
            end
            if s.hits[1] then
                (s.hits[1] :: Object).timer = 1
            end
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080CBFAC: KendoMan on (x, y) for `controller`, of its side, with its element and the damage word. (His position
-- is the spawner's registers: the row, the element and 0.)
function kendoman.summon(controller: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(kendoman.kind, Vec3.new(y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    ;(o.state :: State).controller = controller
    return o
end

return kendoman
