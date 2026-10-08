--!strict
-- CopyDmg's mark (attack object #0x28 in all three games: EXE6's
-- `sub_80C8FFC`, EXE5's 0x080C9A6C, EXE4's 0x080D1C24), made for a game's
-- look (`mark.make`): an unseen probe that steps a panel a tick along its
-- row until it hits something. If that was a navi (a player on the panel,
-- found in the other side's actor list), it marks the navi for its game's
-- time: the navi's side's damage-carry record makes it take, each tick, the
-- damage its side took the tick before on top of its own (the intake's
-- `sub_802CE10`). It goes when its time is up or the record is no longer
-- its. A probe that hits nothing, or leaves the field, or finds time up,
-- just goes.
--
-- How the mark sits over the navi is its game's (`over_navi`): EXE6's and
-- EXE5's at the navi's attach point 0x1B, shown as long as the navi is
-- drawn, for 180 ticks; EXE4's at the navi's position, shown from the mark
-- on, for 90 ticks (and, under event flag 0x1187, which no netbattle sets,
-- a hit of 50 on its panel as it marks: 0x080D1D06).
--
-- EXE5's also hits its panel for 50 each tick it marks, in the own-gauges
-- mode (battle flag 0x40, EXE5's operation battle: 0x080C9B90); EXE5's and
-- EXE4's reload its animation at its init.
--
-- Four states: init, flying (update), marking (destroy) and gone
-- (finish).

local regions = require("@exelib/regions")

local mark = {}

-- The attach point the mark sits at, on the marked navi.
local MARK_POINT = 0x1B
-- A panel with a player navi on it (either side's player bit).
local PLAYERS = 0x60_0000
-- The probe's element: the sixth slot, which no hit's element names.
local SIXTH_ELEMENT = 5
-- EXE5's hit in the own-gauges mode: 50, hit modifier 3.
local OWN_GAUGES_DAMAGE, OWN_GAUGES_HIT_MOD = 50, 3

-- The damage-carry record the probe's side sets: the other side's (the
-- one that takes the damage).
local function carry_side(me: Object): number
    return if me.alliance == 0 then 1 else 0
end

-- `sub_800AE54`: the first of `side`'s actors standing on (x, y).
local function actor_at(x: number, y: number, side: number): Object?
    for _, o in battle.alive_actors(side) do
        if o.panel_x == x and o.panel_y == y then
            return o
        end
    end
    return nil
end

-- `sub_80C913C`: mark the navi on the probe's panel, if a player stands
-- there: it becomes the damage-carry record's target, with the probe as
-- its source.
local function mark_navi(me: Object): boolean
    local x, y = me.panel_x, me.panel_y
    if not bit32.btest(field.flags(x, y), PLAYERS) then
        return false
    end
    local target = actor_at(x, y, 1 - me.alliance)
    if target == nil then
        return false
    end
    me.related1 = target
    battle.set_damage_carry(carry_side(me), { this_tick = 0, previous = 0, source = me, target = target })
    return true
end

-- Gone: hidden, then freed.
local function vanish(me: Object)
    me.visible = false
    me:set_lifecycle("finish")
end

-- A mark of `look`'s game: its kind and `spawn`.
function mark.make(look: CopyDamageMarkLook): CopyDamageMark
    -- `sub_80C9074` (EXE5's 0x080C9AE8): resolve its hits; on a hit, mark
    -- (or go); else a panel on.
    local function fly(me: Object)
        local c = me.collision
        c:remove()
        if c.hit_flags ~= 0 then
            battle.play_sound(look.sound)
            c:set_region(nil)
            if mark_navi(me) then
                me.anim = look.anim
                me.anim_loaded = 0xFF
                if look.over_navi == "position" then
                    me.visible = true
                end
                me.timer = look.ticks
                if look.over_navi == "position" then
                    me.pos = (me.related1 :: Object).pos
                end
                me:set_lifecycle("destroy")
                return
            end
            return vanish(me)
        end
        if battle.time_up() then
            return vanish(me)
        end
        -- Its X velocity is a panel step (±1).
        me.panel_x = me.panel_x + me.vel.x
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        if not field.valid(me.panel_x, me.panel_y) then
            return vanish(me)
        end
        c:present()
    end

    -- `sub_80C90DC` (EXE5's 0x080C9B50, EXE4's 0x080D1D40): over the
    -- marked navi while the record is the probe's, for its time (EXE5's
    -- hitting its panel each tick in the own-gauges mode); then the record
    -- forgets it (`sub_802CDE6`) and it goes.
    local function marking(me: Object)
        local target = me.related1 :: Object
        local at_attach_point = look.over_navi == "attach_point"
        if at_attach_point then
            me:copy_visibility(target)
        end
        local side = carry_side(me)
        local rec = battle.damage_carry(side)
        if rec.source == me and rec.target ~= nil then
            if at_attach_point then
                -- The attach point as the navi's table has it (the API's
                -- faces the navi's way), mirrored the probe's way. (For
                -- NameIDs 0xCD to 0xFF the game would read another table;
                -- only navis get marked.)
                local ax, ay = target:attach_point(MARK_POINT)
                ax *= target.facing
                local dx = int.mul32(ax * 0x10000, -me.facing)
                me.pos = target.pos + Vec3.new(dx, 0, ay * 0x10000)
            else
                me.pos = target.pos
            end
            local hit = look.own_gauges_hit
            if hit and battle.own_gauges() then
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0,
                    region = regions.single,
                    hit_effect = hit.spark,
                    target = hit.hits,
                    self_type = hit.is,
                    damage = OWN_GAUGES_DAMAGE,
                    stamina = 0,
                    hit_mod = OWN_GAUGES_HIT_MOD,
                })
            end
            local t = me.timer - 1
            me.timer = t
            if t ~= 0 then
                return
            end
        end
        if rec.source == me then
            local forgotten: DamageCarry =
                { this_tick = rec.this_tick, previous = rec.previous, source = nil, target = nil }
            battle.set_damage_carry(side, forgotten)
        end
        vanish(me)
    end

    -- `sub_80C9024` (EXE5's 0x080C9A94, EXE4's 0x080D1C4C).
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        me.anim = look.anim
        if look.reload_anim then
            me.anim_loaded = 0xFF
        end
        sprite:set_animation(look.anim)
        sprite.shadow = "with_sprite"
        sprite:step()
        me:set_coordinates_from_panel()
        me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
        me.element = SIXTH_ELEMENT
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.is, look.hits, 0)
        me.collision:present()
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        -- `sub_80C8FFC` (EXE5's 0x080C9A6C, EXE4's 0x080D1C24).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
                if not me.active then
                    return
                end
            elseif l == "update" then
                fly(me)
            elseif l == "destroy" then
                marking(me)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }

    -- `sub_80C9184` (EXE5's 0x080C9C14, EXE4's 0x080D1DB4): a probe on
    -- (x, y) for `owner`, with the damage word `damage`. It goes at the end
    -- of the update list (`sub_8003374`). Its position is the spawner's
    -- registers (the panel Y, the element and 0) until its init places it.
    local function spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
        local o = battle.spawn_at_end(kind, Vec3.new(y, element, 0))
        if o then
            o.panel_x, o.panel_y = x, y
            o:set_damage_word(damage)
            o.alliance, o.flip = owner.alliance, owner.flip
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return mark
