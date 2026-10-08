--!strict
-- ShadeMan's crush, as EXE5 and EXE4 share it (attack object #0x34: EXE5's 0x080CB4AC, EXE4's 0x080D3748; their own,
-- the same code but for what the game's spec states): his figure (his sprite's animation 11), 18 pixels up, that hits
-- its panel at once and later the panel ahead with the ones above and below it (running while dimmed, no spark), for
-- 60 ticks. His command picks what the hits carry (`ShadeManHit`): his figure's palette, a hit modifier, a status, a
-- NaviCust bug.
--
-- Each game makes its kind (`crush.make`): its look and sound, where the figure stands from its panel (EXE5's 40
-- pixels ahead; EXE4's 8 pixels left and 12 up), when the second hit comes (EXE5's with 50 ticks left, EXE4's 40),
-- and EXE5's highlight of the panels it reaches, each tick from the first.

local regions = require("@exelib/regions")

local crush = {}

type State = {
    -- What his command gave (his +0x70: the crush's parameters).
    palette: number,
    hit_mod: number,
    status: Status?,
    bug: number,
    bug_arg: number,
}

local TICKS = 0x3C

function crush.make(spec: ShadeManCrushSpec): ShadeManCrush
    -- EXE5's 0x080CB5A2, EXE4's 0x080D3828: a hit on (x, y) over `region` (running while dimmed, no spark), carrying
    -- the command's hit.
    local function hit(me: Object, x: number, y: number, region: Region)
        local s = me.state :: State
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.element,
            z = me.pos.z,
            region = region,
            hit_effect = nil,
            target = spec.hits,
            self_type = spec.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = s.hit_mod,
            status = s.status,
            bug = s.bug,
            bug_arg = s.bug_arg,
        })
        if box then
            box.run_while_dimmed = true
        end
    end

    -- `object_highlightPanelRegion`: the region's panels around (x, y), turned its way (drawn only).
    local function highlight(me: Object, region: Region)
        for _, o in region.panels or {} do
            field.highlight(me.panel_x + o[1] * me.facing, me.panel_y + o[2])
        end
    end

    -- EXE5's 0x080CB538, EXE4's 0x080D37CE: 60 ticks; the second hit, on the panel ahead (and those above and below);
    -- EXE5's panel and the column ahead highlighted each tick, from the first (0x080CB568: region 0x15, a literal,
    -- turned its way). (Its end clears a hit region it never had: `object_clearCollisionRegion` through no collision
    -- data.)
    local function tick(me: Object)
        if battle.over() then
            me:set_lifecycle("destroy")
            return
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left <= 0 then
            me:set_lifecycle("destroy")
            return
        end
        if me.timer == spec.second_at then
            hit(me, me.panel_x + me.facing, me.panel_y, regions.wide)
        end
        if spec.highlights then
            highlight(me, regions.fan)
        end
    end

    -- EXE5's 0x080CB4D0, EXE4's 0x080D376C: his figure (drawn with its sprite), its palette by the command, where the
    -- game stands it from its panel; the hit on its panel; 60 ticks.
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(spec.sprite)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = 11, 11
        sprite:set_animation(11)
        sprite.palette = s.palette
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.timer = TICKS
        me:set_coordinates_from_panel()
        spec.place(me)
        hit(me, me.panel_x, me.panel_y, regions.single)
        battle.play_sound(spec.sound)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = spec.id,
        pool = "attack",
        state = { palette = "u8", hit_mod = "u8", status = "status", bug = "u8", bug_arg = "u8" },
        -- EXE5's 0x080CB4AC, EXE4's 0x080D3748.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
            me:update_sprite()
        end,
    }

    -- EXE5's 0x080CB57C, EXE4's 0x080D3802: the crush of `shademan` on (x, y), 18 pixels up, with his damage word and
    -- what his command gave.
    local function spawn(shademan: Object, x: number, y: number, given: ShadeManHit): Object?
        local o = battle.spawn(kind, Vec3.new(y, 0, 0x12_0000))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = 0
            o:set_damage_word(bit32.bor(shademan.damage, bit32.lshift(shademan.stamina, 16)))
            o.alliance, o.flip = shademan.alliance, shademan.flip
            o.run_while_dimmed = true
            local s = o.state :: State
            s.palette, s.hit_mod, s.status = given.palette, given.hit_mod, given.status
            s.bug, s.bug_arg = given.bug or 0, given.bug_arg or 0
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return crush
