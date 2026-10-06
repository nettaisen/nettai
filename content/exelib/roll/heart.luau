--!strict
-- Roll's heart, as EXE6 and EXE5 share it (the original's attack object #0x4E: EXE6's `sub_80CE118`, EXE5's 0x080CF71C,
-- the same code but for its sprite and the effect table's rows): it sinks from 160 pixels above her user's panel, 3
-- pixels a tick, sparkling every 10 ticks; landed, it heals the user by its damage (or to full, with none), with a
-- sound and an effect, and goes 60 ticks later. See docs/engine/chips.md §3.6.20.
--
-- Each game makes its own kind (`heart.make`): its sprite and animation, its sound, and the sparkle (the effect
-- table's row 0x26) and the effect on the healed navi (row 6).

local heart = {}

export type Spec = {
    -- The kind's id.
    id: string,
    sprite: SpriteAsset,
    anim: number,
    heal_sound: Sound,
    sparkle: Effect,
    healed: Effect,
}

export type Heart = {
    kind: Kind,
    spawn: (user: Object, x: number, y: number, amount: number) -> Object?,
}

-- Its height, and how far it sinks a tick (Z's whole part).
local HEIGHT = 0xA0
local SINK = 3

function heart.make(spec: Spec): Heart
    -- `sub_80CE198`: sink, sparkling every 10 ticks; landed, hidden, on to the heal.
    local function sink(me: Object)
        local t = me.timer + 1
        me.timer = t
        if t % 10 == 0 then
            battle.effect(battle.jitter(0x10, me.pos), spec.sparkle)
        end
        local p = me.pos
        local z = bit32.rshift(bit32.band(p.z, 0xFFFF_0000), 16) - SINK
        if z >= 0 then
            me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), z * 0x10000))
            return
        end
        me.visible = false
        me:set_action(4)
    end

    -- `sub_80CE1D6`: heal the user (`object_addHP`), then 60 ticks.
    local function heal(me: Object)
        if me.phase == 0 then
            local user = me.related1 :: Object
            local amount = me.damage
            if amount == 0 then
                amount = user.max_hp
            end
            user.hp = math.min(user.hp + amount, user.max_hp)
            battle.play_sound(spec.heal_sound)
            battle.effect(user.pos, spec.healed)
            me.timer = 0x3C
            me.phase = 4
            return
        end
        local left = me.timer - 1
        me.timer = left
        if left ~= 0 then
            return
        end
        me:set_lifecycle("destroy")
    end

    local ACTIONS = { [0] = sink, [4] = heal }

    -- `sub_80CE178`.
    local function tick(me: Object)
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("Roll's heart action %#x reads past its table (off_80CE190)", me.action))
        end
        action(me)
        me:update_sprite()
    end

    -- `sub_80CE138`: high over its panel; then its first update at once.
    local function init(me: Object)
        me.visible = true
        local sprite = me.sprite
        sprite:load(spec.sprite)
        me.anim = spec.anim
        me.anim_loaded = 0
        sprite:set_animation(spec.anim)
        sprite:step()
        sprite.shadow = "with_sprite"
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, HEIGHT * 0x10000)
        me.timer = 0
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = spec.id,
        pool = "attack",
        -- `sub_80CE118`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
        end,
    }

    -- `sub_80CE224`: a heart over (x, y) for `user`, healing `amount`. (Its position is the spawner's registers, all
    -- of which its init overwrites.)
    local function spawn(user: Object, x: number, y: number, amount: number): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if o then
            o.panel_x, o.panel_y = x, y
            o:set_damage_word(amount)
            o.alliance = user.alliance
            o.related1 = user
            o.run_while_dimmed = true
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return heart
