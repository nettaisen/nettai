--!strict
-- PrixPowr's VideoMan (EXE4's actor object #0x53, 0x080CBB40: EXE4's own, not the VideoMan chips' #0x3E), the first of
-- its three. He appears on the user's panel (his sound) and, 21 ticks later, raises his projector (animation 8, then
-- 9 as it ends: its light, his sheet's animation 18, an attachment, with its sound); 21 ticks into that he projects
-- LaserMan's silhouette on the panel ahead for 60 ticks (./silhouette, its sound); 20 ticks on he lowers it
-- (animation 10, the light gone), and 6 ticks later goes in a puff. He hits nothing. He runs while dimmed.

local attachment = require("../../objects/attachment")
local silhouette = require("./silhouette")

local videoman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His projector's light (the original keeps it in his X velocity).
    held: Object?,
}

local SPRITE = asset.sprite("videoman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_PROJECTOR = asset.sound("projector")
local SOUND_SILHOUETTE = asset.sound("place")
-- The puff he goes in (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
-- His projector's light (attachment row 0x2B: his own sheet, palette 0), in its animation 18, animating while dimmed.
local LIGHT = attachment.look { sprite = SPRITE, palette = 0 }
local LIGHT_ANIM = 18

local APPEAR, PROJECT = 0x0, 0x4

-- Action 0 (0x080CBBF0): 21 ticks; then his projector up (animation 8), 46 ticks.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    me.anim = 8
    me.timer = 45
    me:set_action(PROJECT)
end

-- Action 4 (0x080CBC0C): as his projector is up, animation 9 and its light, its sound; with 25 ticks left LaserMan's
-- silhouette on the panel ahead, its sound; with 5 left animation 10, the light gone (0x080CD708); at its end he
-- goes.
local function project(me: Object)
    if me.anim == 8 and bit32.btest(me.sprite.frame_flags, 0x80) then
        me.anim = 9
        attachment.attach(me, LIGHT, "held", LIGHT_ANIM, true)
        battle.play_sound(SOUND_PROJECTOR)
    end
    if me.timer == 25 then
        silhouette.show(me, me.panel_x + me.facing, me.panel_y, 60)
        battle.play_sound(SOUND_SILHOUETTE)
    end
    if me.timer == 5 then
        me.anim = 10
        local light = (me.state :: State).held
        if light then
            light:set_lifecycle("destroy")
        end
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080CBBD6.
local function tick(me: Object)
    if me.action == APPEAR then
        appear(me)
    elseif me.action == PROJECT then
        project(me)
    else
        error(string.format("PrixPowr's VideoMan's action %#x reads past his table (0x080CBBE8)", me.action))
    end
end

-- 0x080CBB84: his sprite (08-17) with his shadow, shown, in animation 0, palette 0, facing his side's way, on his
-- panel; his sound; 20 ticks; then his first tick.
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
    battle.play_sound(SOUND_APPEAR)
    me.timer = 20
    me:set_lifecycle("update")
    tick(me)
end

videoman.kind = new.kind {
    id = "prixpowr/videoman",
    pool = "actor",
    state = { controller = "object", held = "object" },
    -- 0x080CBB40: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080CBB64: a puff 16 pixels above him; the controller moves on; he is freed.
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

-- 0x080CBC8A: VideoMan on (x, y) for `controller`, of its side, with its element and the damage word. (His position
-- is the spawner's registers: the row, the element and 0.)
function videoman.summon(controller: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(videoman.kind, Vec3.new(y, controller.element, 0))
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

return videoman
