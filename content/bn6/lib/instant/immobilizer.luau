--!strict
-- An immobilizing hit (the original's attack object #0x3F, `sub_80CC044`):
-- on its first update a one-tick hit on its panel that deals nothing but
-- immobilizes (status 0x42), then 20 ticks before it goes. Instant chip
-- effect 9 spawns it (immobilize.luau).

local collision = require("../../rules/collision")
local regions = require("@common/regions")
local statuses = require("../../rules/status")

local immobilizer = {}

-- The hit (`dword_80CC0C8`, `dword_80CC0CC`): one panel, no spark, a
-- guard-breaking attack against navis, with status 0x42.
local NO_SPARK: Spark? = nil
local STATUS = statuses.immobilize_120

local SOUND = asset.sound("bn6:immobilizer")

-- `sub_80CC090`: 20 ticks, then gone.
local function linger(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x14
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
    me.sprite:step()
end

-- `sub_80CC064`: on its panel, on the ground; the hit, then the wait.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = NO_SPARK,
        target = collision.hits_navis,
        self_type = collision.guard_breaking,
        damage = 0,
        stamina = 0,
        status = STATUS,
    })
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    linger(me)
end

immobilizer.kind = define.kind {
    id = "bn6:immobilizer",
    pool = "attack",
    -- `sub_80CC044`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            linger(me)
        else
            me:free()
        end
    end,
}

-- `sub_80CC0B2` with `sub_801155A`: one on (x, y) for `owner`, no element
-- and no damage. (Its spawn position is the spawner's registers; its init
-- places it before anything reads it.)
function immobilizer.spawn(owner: Object, x: number, y: number): Object?
    local o = battle.spawn(immobilizer.kind, Vec3.new(y, 0, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(0)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return immobilizer
