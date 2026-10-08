--!strict
-- The hole over a navi under the ground (EXE4's effect object #0x3E, 0x080E64FC; EXE5's ripple over a body under its
-- sea, EXE6's `sub_80E4CB8`): the navi's tail keeps one while its flags say it is under (flag 4, 0x0800C0A6: the role
-- `kinds.dive_ripple`, the navi its first related; PopUp's glow puts it under) and ends it as it comes up. It sits on
-- the ground under the navi, its animation by the navi's (0 standing, 2 for its 3, 1 for its 4 and any other), seen
-- by whoever sees its side (0x0800B0CA). (Navi 12's, WoodMan's, stays at 0: the story's, never a netbattle's. Unlike
-- EXE5's it brings no splash.)

local hole = {}

local SPRITE = asset.sprite("underground-hole")

-- 0x080E6562: under the navi, on the ground.
local function follow(me: Object)
    local navi = me.related1 :: Object
    local p = navi.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local a = navi.anim
    me.anim = if a == 0 then 0 elseif a == 3 then 2 else 1
    me.visible = true
    me:hide_from_blind()
end

hole.kind = new.kind {
    id = "underground-hole",
    pool = "effect",
    -- 0x080E64FC: its state's routine, then its sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E6520: its sprite (0C-2D) with its shadow, all of it under the objects, animation 0 from its first
            -- frame, stepped once. (Its call of 0x0800307C, EXE6's `sub_800302A`, with 1 sets bits of the sprite
            -- nothing that draws one reads.)
            me.visible = true
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "ground"
            sprite.under_objects = true
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite:step()
            me:set_lifecycle("update")
            follow(me)
        elseif l == "update" then
            follow(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

return hole
