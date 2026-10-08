--!strict
-- The element flame, as EXE5 and EXE4 share it (EXE5's attack object #0x98, 0x080D99D8, ElemRage's and the fire arm's;
-- EXE4's #9, 0x080CE2D8, the Elem chips': the same code but for its constants and the spread): a flame on its panel
-- that, when its animation first ends, hits its panel and sends the next one ahead until its count (Param1's low seven
-- bits) is spent, and stands its time (Param2). Each game makes its kind (`flame.make_kind`) with its look: its
-- sprite, the panel types its flames may leave (Param4's high nibble names one: the flame's `leaves`), its collision
-- types and spark, and whether a flame with Param1's bit 7 also sends the two beside (EXE5's; EXE4's keeps the bit and
-- sends none).

local regions = require("../regions")

local flame = {}

type State = ElemFlameState
type Look = ElemFlameLook

-- A flame that leaves no panel type, as its state holds it (Param4's high nibble 0).
local NONE = "none"

-- A game's flame of `look`.
function flame.make_kind(look: Look): ElemFlame
    local spawn_flame: (owner: Object, x: number, y: number, element: number, z: number, s: State, damage: number) -> Object?

    -- Action 0 (EXE5's 0x080D9B50, EXE4's 0x080CE44C): when its animation ends: its idle animation, its hit on; the
    -- first time, the next flame ahead (and, spreading, the two beside it), its count one less.
    local function burst(me: Object)
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        local s = me.state :: State
        me.anim = 0
        me.collision:set_region(regions.single)
        if not s.sent then
            s.sent = true
            if s.count > 1 then
                s.count -= 1
                local x, y = me.panel_x + me.facing, me.panel_y
                local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
                local function next_flame(paralyzes: boolean, spread: boolean): State
                    return {
                        count = s.count,
                        spread = spread,
                        ticks = s.ticks,
                        offset = s.offset,
                        paralyzes = paralyzes,
                        attack_480 = s.attack_480,
                        while_dimmed = s.while_dimmed,
                        leaves = s.leaves,
                        palette = s.palette,
                        hit = false,
                        sent = false,
                    }
                end
                -- (EXE4's keeps Param1's bit 7 on the next one; EXE5's sends it without.)
                local o = spawn_flame(me, x, y, me.element, me.pos.z, next_flame(s.paralyzes, s.spread and not look.spreads), damage)
                if o then
                    o.related1 = me.related1
                end
                if s.spread and look.spreads then
                    -- (The ones beside, above then below, without Param4's bit 2: not paralyzing; the rest kept.)
                    for _, dy in { -1, 1 } do
                        local q = spawn_flame(me, x, y + dy, me.element, me.pos.z, next_flame(false, false), damage)
                        if q then
                            q.related1 = me.related1
                        end
                    end
                end
            end
        end
        me:set_action(4)
    end

    -- EXE5's 0x080D9AA0, EXE4's 0x080CE39E: its panel's type left at 3 ticks to go; its time out (or its owner gone),
    -- it ends; a hit takes its region (a paralyzing one back 12 ticks later, hit modifier 3 then); its action; its
    -- panel highlighted while it can hit.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        local s = me.state :: State
        if me.timer == 3 and s.leaves ~= NONE then
            field.set_type(me.panel_x, me.panel_y, s.leaves :: PanelType)
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        local owner = me.related1
        local ended = t == 0 or (owner ~= nil and not owner.active)
        if not ended then
            if c.hit_flags ~= 0 then
                s.hit = true
                me.timer2 = 0
                c.hit_mod_base = 3
                c:set_region(nil)
            end
            ended = battle.over()
        end
        if ended then
            c:set_region(nil)
            me:set_lifecycle("destroy")
            me.visible = false
            return
        end
        if me.action == 0 then
            burst(me)
        elseif me.action ~= 4 then
            error(string.format("%s: action %#x reads past its table", look.id, me.action))
        end
        if s.paralyzes and s.hit then
            me.timer2 = int.u16(me.timer2 + 1)
            if me.timer2 == 12 then
                c:set_region(regions.single)
            end
        end
        if c.region ~= nil then
            field.highlight(me.panel_x, me.panel_y)
        end
        c:present()
    end

    -- EXE5's 0x080D9A08, EXE4's 0x080CE308: off the field, gone; else shown on its panel (its X by its offset
    -- ahead), its time, its palette, its collision (an attack, or row 0x25, that hits navis: hit modifier 3, or 1
    -- paralyzing), its element's spark, no region yet; the first update at once.
    local function init(me: Object)
        if not field.valid(me.panel_x, me.panel_y) then
            me:free()
            return
        end
        local s = me.state :: State
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0xFF
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + int.shl(me.facing * s.offset, 16)), p.y, p.z)
        me.timer = s.ticks
        s.hit, s.sent = false, false
        sprite.palette = s.palette
        if not me:create_collision() then
            me:free()
            return
        end
        local hit_mod = if s.paralyzes then 1 else 3
        local is = if s.attack_480 then look.attack_480 else look.attack
        me:setup_collision(is, look.hits, hit_mod)
        me.collision:set_hit_effect(look.spark(me.element))
        me.collision:set_region(nil)
        me.collision:present()
        me:set_lifecycle("update")
        tick(me)
    end

    local leaves = { NONE }
    for _, t in look.leaves or {} do
        table.insert(leaves, t)
    end
    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = {
            count = "u8",
            spread = "bool",
            ticks = "u8",
            offset = "i8",
            paralyzes = "bool",
            attack_480 = "bool",
            while_dimmed = "bool",
            leaves = leaves,
            palette = "u8",
            hit = "bool",
            sent = "bool",
        },
        -- EXE5's 0x080D99D8, EXE4's 0x080CE2D8: by its lifecycle, then a sprite step (while dimmed too and whatever
        -- its collision holds, with any of Param4's flags or a panel type to leave: the game tests the whole byte).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
                if not me.active then
                    return
                end
            elseif l == "update" then
                tick(me)
            else
                me:free()
                return
            end
            local s = me.state :: State
            if s.while_dimmed or s.paralyzes or s.attack_480 or s.leaves ~= NONE then
                me:update_sprite_while_dimmed()
            else
                me:update_sprite()
            end
        end,
    }

    -- EXE5's 0x080D9BE6, EXE4's 0x080CE49C.
    spawn_flame = function(owner: Object, x: number, y: number, element: number, z: number, s: State, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(y, element, z))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = nil
        local t = o.state :: State
        t.count, t.spread, t.ticks, t.offset, t.palette = s.count, s.spread, s.ticks, s.offset, s.palette
        t.paralyzes, t.attack_480, t.while_dimmed = s.paralyzes, s.attack_480, s.while_dimmed
        t.leaves = s.leaves or NONE
        if s.while_dimmed then
            o.run_while_dimmed = true
        end
        return o
    end

    return { kind = kind, spawn = spawn_flame }
end

return flame
