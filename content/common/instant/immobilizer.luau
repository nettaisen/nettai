--!strict
-- An immobilizing hit, as BN5 and BN6 share it (BN6's attack object #0x3F,
-- `sub_80CC044`; BN5's #0x3F, 0x080CD2D8, the same code but that it shows
-- the immobilized mark): on its first update a one-tick hit on its panel
-- that deals nothing but immobilizes (status 0x42, the game's), then 20
-- ticks before it goes. Each game makes its kind
-- (`immobilizer.make_kind(id, look)`); BN6's instant chip effect 9
-- (lib/instant/immobilize) and BN5's MagnetSoul's B+Back spawn it.

local regions = require("@common/regions")

local immobilizer = {}

-- `sub_80CC090`: 20 ticks, then gone (BN5's hidden first).
local function linger(me: Object, look: ImmobilizerLook)
    if me.phase_init == 0 then
        me.timer = 0x14
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        if look.sprite then
            me.visible = false
        end
        me:set_lifecycle("destroy")
    end
    me.sprite:step()
end

-- `sub_80CC064`: on its panel, on the ground (BN5's showing its mark); the
-- hit (`dword_80CC0C8`, `dword_80CC0CC`: one panel, no spark, a
-- guard-breaking attack against navis, the status), then the wait.
local function init(me: Object, look: ImmobilizerLook)
    local mark = look.sprite
    if mark then
        local sprite = me.sprite
        sprite:load(mark)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = look.anim or 0, look.anim or 0
        sprite:set_animation(look.anim or 0)
        sprite:step()
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if mark then
        me.visible = true
    end
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = look.target,
        self_type = look.self_type,
        damage = 0,
        stamina = 0,
        status = look.status,
    })
    battle.play_sound(look.sound)
    me:set_lifecycle("update")
    linger(me, look)
end

-- A game's immobilizer kind (`id`) of `look`.
function immobilizer.make_kind(id: string, look: ImmobilizerLook): Kind
    return define.kind {
        id = id,
        pool = "attack",
        -- `sub_80CC044`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                linger(me, look)
            else
                me:free()
            end
        end,
    }
end

-- `sub_80CC0B2` with `sub_801155A`: one of `kind` (a game's) on (x, y) for
-- `owner`, no element and no damage. (Its spawn position is the spawner's
-- registers; its init places it before anything reads it.)
function immobilizer.spawn(kind: Kind, owner: Object, x: number, y: number): Object?
    local o = battle.spawn(kind, Vec3.new(y, 0, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(0)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_8015AA6`'s loop: from (x, y) forward to the field's edge, an
-- immobilizer of `kind` on every panel with an enemy body (`off_8015AF8`,
-- by the user's side).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
function immobilizer.ahead(kind: Kind, user: Object, x: number, y: number)
    local ahead = user.facing
    local enemy = ENEMY_BODY[user.alliance]
    while field.valid(x, y) do
        if bit32.btest(field.flags(x, y), enemy) then
            immobilizer.spawn(kind, user, x, y)
        end
        x += ahead
    end
end

return immobilizer
