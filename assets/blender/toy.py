"""Deterministic toy shell. Run from the repository root with Blender --background.

All geometry starts as Blender primitives; bevels and Boolean modifiers form
the shell and recesses. Dimensions are meters, colors are sRGB. No input files,
textures, random values, external assets, or window-dependent operators.
"""

import json
import math
import os
from pathlib import Path
import struct
import sys

import bmesh
import bpy
from mathutils import Euler, Matrix, Quaternion, Vector


PARAMS = {
    "body_size": (0.250, 0.220, 0.040),
    "nose_height": 0.016,
    "body_bevel": 0.008,
    "hood_size": (0.216, 0.108, 0.045),
    "hood_center": (0.0, 0.054, 0.042),
    "hood_rise": 0.042,
    "fender_size": (0.036, 0.190, 0.060),
    "fender_centers": ((-0.106, 0.010, 0.040), (0.106, 0.010, 0.040)),
    "rear_hump_size": (0.202, 0.046, 0.043),
    "rear_hump_center": (0.0, 0.026, 0.0685),
    "spoiler_size": (0.214, 0.009, 0.008),
    "spoiler_center": (0.0, 0.047, 0.090),
    "console_size": (0.213, 0.088, 0.025),
    "console_center": (0.0, -0.064, 0.0525),
    "dash_top_size": (0.228, 0.057, 0.015),
    "dash_top_center": (0.0, -0.023, 0.073),
    "windshield_size": (0.20, 0.06, 0.016),
    "windshield_center": (0.0, 0.003, 0.0735),
    "side_vent_size": (0.002, 0.026, 0.013),
    "side_vent_centers": ((-0.124, -0.025, 0.037), (0.124, -0.025, 0.037)),
    "side_vent_rake_deg": 15.0,
    "screen_size": (0.190, 0.11875),
    "screen_center": (0.0, 0.003, 0.1465),
    "screen_tilt_deg": 12.0,
    "frame_size": (0.214, 0.142, 0.018),
    "frame_bevel": 0.004,
    "screen_back_depth": 0.006,
    # ~5 cm radius gives the broad wheel seen in the reference photographs.
    "wheel_radius": 0.050,
    "wheel_tube_radius": 0.0055,
    "wheel_major_segments": 64,
    "wheel_minor_segments": 12,
    "wheel_center": (0.0, -0.136, 0.066),
    "wheel_tilt_from_vertical_deg": 25.0,
    "column_length": 0.090,
    "column_radius": 0.007,
    "wheel_hub_radius": 0.014,
    "wheel_hub_depth": 0.010,
    "wheel_spoke_width": 0.008,
    "wheel_spoke_depth": 0.005,
    "wheel_spoke_angles_deg": (30.0, 150.0, 270.0),
    "lever_base": (0.100, -0.074, 0.067),
    "lever_length": 0.045,
    "lever_radius": 0.0027,
    "lever_knob_radius": 0.008,
    "lever_travel_deg": 20.0,
    "lever_socket_radius": 0.009,
    "lever_socket_depth": 0.004,
    "key_slot": (-0.098, -0.092, 0.070),
    "key_radius": 0.003,
    "key_length": 0.010,
    "key_grip_size": (0.004, 0.016, 0.007),
    "lamp_centers": ((-0.069, -0.088, 0.056), (0.069, -0.088, 0.056)),
    "lamp_size": (0.048, 0.036, 0.032),
    "lamp_lens_size": (0.033, 0.002, 0.014),
    "lamp_rake_deg": -12.0,
    "bumper_center": (0.0, 0.1135, 0.013),
    "bumper_size": (0.228, 0.014, 0.022),
    "running_light_size": (0.021, 0.002, 0.006),
    "running_light_spacing": 0.074,
    "dash_center": (0.0, -0.045, 0.064),
    "dash_rotation_x_deg": 55.0,
    "dash_size": (0.174, 0.034, 0.007),
    "gauge_centers_x": (-0.051, -0.013, 0.025),
    "gauge_radii": (0.012, 0.010, 0.007),
    "gauge_recess_depth": 0.003,
    "turbo_lamp_center_x": 0.066,
    "turbo_lamp_radius": 0.0025,
    "cylinder_segments": 32,
    "knob_segments": 24,
    "knob_rings": 12,
    "bevel_segments": 3,
    "detail_bevel": 0.001,
    "triangle_budget": 60000,
    "materials": {
        "MatBodyRed": {"color": (0.82, 0.07, 0.06), "roughness": 0.35, "metallic": 0.0},
        "MatConsoleRed": {"color": (0.82, 0.07, 0.06), "roughness": 0.35, "metallic": 0.0},
        "MatCockpitBlack": {"color": (0.05, 0.05, 0.05), "roughness": 0.70, "metallic": 0.0},
        "MatChrome": {"color": (0.8, 0.8, 0.85), "roughness": 0.20, "metallic": 0.9},
        "MatTireBlack": {"color": (0.025, 0.025, 0.025), "roughness": 0.80, "metallic": 0.0},
        "MatLampLens": {"color": (0.1, 0.12, 0.16), "roughness": 0.15, "metallic": 0.0},
        "MatScreenDark": {"color": (0.045, 0.055, 0.07), "roughness": 0.35, "metallic": 0.0},
    },
}

PIVOTS = {
    "Body": "Base footprint center at ground level; fixed.",
    "ScreenFrame": "Screen recess center; local XY is screen plane, +Z faces player.",
    "Screen": "Same origin and rotation as ScreenFrame; centered 0.15 x 0.095 m quad.",
    "SteeringWheel": "Wheel hub center; local Z spin axis; plane tilted 25 degrees from vertical, top away.",
    "GearLever": "Console lever base, right of wheel; shaft along local +Y; tilt +/-20 degrees about local X.",
    "IgnitionKey": "Dash key slot, left of wheel; insertion and twist axis is local X.",
    "HeadlightLeft": "Left pop-up lamp pod base; negative X is player's left.",
    "HeadlightRight": "Right pop-up lamp pod base; positive X is player's right.",
    "Bumper": "Base front, centered on dark bumper strip.",
    "DashPanel": "Dashboard face center, on front surface of instrument panel.",
}


def activate(obj):
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj


def apply_modifier(obj, modifier):
    activate(obj)
    bpy.ops.object.modifier_apply(modifier=modifier.name)


def bevel(obj, width):
    if width:
        modifier = obj.modifiers.new("Molded edges", "BEVEL")
        modifier.width = width
        modifier.segments = PARAMS["bevel_segments"]
        apply_modifier(obj, modifier)
    return obj


def assign(obj, material):
    if material:
        obj.data.materials.append(bpy.data.materials[material])
    return obj


def cube(size, location=(0, 0, 0), material=None, rounding=0, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(size=1, location=location, rotation=rotation)
    obj = bpy.context.object
    obj.scale = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return assign(bevel(obj, rounding), material)


def cylinder(radius, depth, location=(0, 0, 0), material=None):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=PARAMS["cylinder_segments"], radius=radius, depth=depth,
        end_fill_type="NGON", location=location,
    )
    obj = bpy.context.object
    for face in obj.data.polygons:
        face.use_smooth = len(face.vertices) == 4
    return assign(obj, material)


def boolean(obj, cutter, operation):
    modifier = obj.modifiers.new(operation, "BOOLEAN")
    modifier.operation = operation
    modifier.solver = "EXACT"
    modifier.object = cutter
    apply_modifier(obj, modifier)
    bpy.data.objects.remove(cutter, do_unlink=True)


def finish(name, pieces, location=(0, 0, 0), rotation=(0, 0, 0)):
    """Bake primitive transforms into vertices, then set the actual part pivot.

    This preserves the requested local rotation axes; moving just the origin
    after a world-space join would lose the steering column's tilted axis.
    """
    activate(pieces[0])
    for obj in pieces:
        obj.select_set(True)
    if len(pieces) > 1:
        bpy.ops.object.join()
    obj = pieces[0]
    obj.data.transform(obj.matrix_world.copy())
    obj.matrix_world = Matrix.Identity(4)
    obj.name = name
    obj.data.name = name + "_Mesh"
    obj.location = location
    obj.rotation_euler = rotation
    obj["pivot_description"] = PIVOTS[name]
    return obj


def srgb_linear(value):
    return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4


def create_materials():
    for name, values in PARAMS["materials"].items():
        material = bpy.data.materials.new(name)
        material.use_nodes = True
        color = tuple(srgb_linear(c) for c in values["color"]) + (1.0,)
        material.diffuse_color = color
        shader = material.node_tree.nodes.get("Principled BSDF")
        shader.inputs["Base Color"].default_value = color
        shader.inputs["Roughness"].default_value = values["roughness"]
        shader.inputs["Metallic"].default_value = values["metallic"]


def build_parts():
    p = PARAMS
    red, black, chrome = "MatBodyRed", "MatCockpitBlack", "MatChrome"
    screen_rot = (math.radians(90 - p["screen_tilt_deg"]), 0, 0)
    column_rot = (math.radians(90 - p["wheel_tilt_from_vertical_deg"]), 0, 0)
    dash_rot = (math.radians(p["dash_rotation_x_deg"]), 0, 0)
    screen_basis = Euler(screen_rot).to_matrix()
    column_basis = Euler(column_rot).to_matrix()
    parts = {}

    # Start with a cube, taper the nose and slope its top before beveling.
    # +Y is the car's nose (away from the seated player); no external wheels.
    body = cube(p["body_size"], (0, 0, p["body_size"][2] / 2), red)
    for vertex in body.data.vertices:
        if vertex.co.y > 0:
            vertex.co.x *= 0.78
            if vertex.co.z > 0:
                vertex.co.z -= p["body_size"][2] - p["nose_height"]
    bevel(body, p["body_bevel"])
    hood = cube(p["hood_size"], p["hood_center"], red)
    for vertex in hood.data.vertices:
        if vertex.co.z > 0:
            vertex.co.z -= vertex.co.y / p["hood_size"][1] * p["hood_rise"]
        if vertex.co.y > 0:
            vertex.co.x *= 0.94
    bevel(hood, p["body_bevel"])
    additions = [body, hood]
    additions.extend(cube(p["fender_size"], center, red, 0.014)
                     for center in p["fender_centers"])
    additions.extend([
        cube(p["rear_hump_size"], p["rear_hump_center"], red, 0.010),
        cube(p["spoiler_size"], p["spoiler_center"], red, 0.003),
        cube(p["console_size"], p["console_center"], "MatConsoleRed", 0.006),
        cube(p["dash_top_size"], p["dash_top_center"], black, 0.004),
        cube(p["windshield_size"], p["windshield_center"], black, 0.003,
             (math.radians(-20), 0, 0)),
    ])
    dash_surround = tuple(additions)
    for center in p["side_vent_centers"]:
        additions.append(cube(p["side_vent_size"], center, "MatTireBlack", 0.0007,
                              (math.radians(p["side_vent_rake_deg"]), 0, 0)))
    column_center = Vector(p["wheel_center"]) + column_basis @ Vector((0, 0, -p["column_length"] / 2))
    column = cylinder(p["column_radius"], p["column_length"], column_center, black)
    column.rotation_euler = column_rot
    additions.append(column)
    # The stationary lever socket belongs to Body, so only the shaft pivots.
    additions.append(cylinder(p["lever_socket_radius"], p["lever_socket_depth"],
                              p["lever_base"], black))
    # Clearance behind the recessed instrument panel, including TurboLamp.
    pocket_depth = p["dash_size"][2] * 3
    pocket_center = Vector(p["dash_center"]) + Euler(dash_rot).to_matrix() @ Vector((0, 0, -pocket_depth / 2))
    for piece in dash_surround:
        boolean(piece, cube((*p["dash_size"][:2], pocket_depth), pocket_center,
                            rotation=dash_rot), "DIFFERENCE")
    parts["Body"] = finish("Body", additions)

    frame = cube(p["frame_size"], material=black, rounding=p["frame_bevel"])
    boolean(frame, cube((*p["screen_size"], p["frame_size"][2] * 3)), "DIFFERENCE")
    bevel(frame, p["detail_bevel"])
    backing = cube((*p["frame_size"][:2], p["screen_back_depth"]),
                   (0, 0, -(p["frame_size"][2] + p["screen_back_depth"]) / 2),
                   black, 0.003)
    parts["ScreenFrame"] = finish("ScreenFrame", [frame, backing], p["screen_center"], screen_rot)
    bpy.ops.mesh.primitive_plane_add(size=1)
    screen = bpy.context.object
    screen.scale = (*p["screen_size"], 1)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    screen.data.materials.append(bpy.data.materials["MatScreenDark"])
    parts["Screen"] = finish("Screen", [screen], p["screen_center"], screen_rot)
    marker = bpy.data.objects.new("SCREEN_MARKER", None)
    bpy.context.collection.objects.link(marker)
    marker.parent = parts["ScreenFrame"]
    marker.empty_display_type = "PLAIN_AXES"
    marker.empty_display_size = 0.01
    marker["purpose"] = "M8 game texture placement; local XY spans the screen."

    bpy.ops.mesh.primitive_torus_add(
        major_segments=p["wheel_major_segments"], minor_segments=p["wheel_minor_segments"],
        major_radius=p["wheel_radius"] - p["wheel_tube_radius"], minor_radius=p["wheel_tube_radius"],
    )
    rim = assign(bpy.context.object, "MatTireBlack")
    for face in rim.data.polygons:
        face.use_smooth = True
    wheel = [rim]
    reach = p["wheel_radius"] - p["wheel_tube_radius"]
    for degrees in p["wheel_spoke_angles_deg"]:
        angle = math.radians(degrees)
        wheel.append(cube(
            (reach, p["wheel_spoke_width"], p["wheel_spoke_depth"]),
            (reach / 2 * math.cos(angle), reach / 2 * math.sin(angle), 0),
            black, p["detail_bevel"], (0, 0, angle),
        ))
    wheel.append(bevel(cylinder(p["wheel_hub_radius"], p["wheel_hub_depth"], material=chrome), p["detail_bevel"]))
    parts["SteeringWheel"] = finish("SteeringWheel", wheel, p["wheel_center"], column_rot)
    parts["SteeringWheel"]["rotation_axis_local"] = "Z"

    shaft_length = p["lever_length"] - p["lever_knob_radius"]
    shaft = cylinder(p["lever_radius"], shaft_length, (0, shaft_length / 2, 0), chrome)
    shaft.rotation_euler.x = math.pi / 2
    lever = [shaft]
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=p["knob_segments"], ring_count=p["knob_rings"],
        radius=p["lever_knob_radius"], location=(0, shaft_length, 0),
    )
    knob = assign(bpy.context.object, "MatTireBlack")
    # UV-sphere primitive face ordering can vary between Blender processes.
    # Canonicalize only the ordering, preserving primitive geometry and UVs.
    topology = bmesh.new()
    topology.from_mesh(knob.data)
    ordered_faces = sorted(topology.faces, key=lambda face: tuple(sorted(tuple(v.co) for v in face.verts)))
    face_order = {face: index for index, face in enumerate(ordered_faces)}
    topology.faces.sort(key=lambda face: face_order[face])
    topology.to_mesh(knob.data)
    topology.free()
    for face in knob.data.polygons:
        face.use_smooth = True
    lever.append(knob)
    parts["GearLever"] = finish("GearLever", lever, p["lever_base"], (math.pi / 2, 0, 0))
    parts["GearLever"]["rotation_axis_local"] = "X"
    parts["GearLever"]["travel_degrees"] = [-p["lever_travel_deg"], p["lever_travel_deg"]]

    key_shaft = cylinder(p["key_radius"], p["key_length"], (p["key_length"] / 2, 0, 0), chrome)
    key_shaft.rotation_euler.y = math.pi / 2
    key = [key_shaft, cube(p["key_grip_size"], (p["key_length"], 0, 0), chrome, p["detail_bevel"])]
    parts["IgnitionKey"] = finish("IgnitionKey", key, p["key_slot"], (0, 0, -math.pi / 2))
    parts["IgnitionKey"]["rotation_axis_local"] = "X"

    for name, center in zip(("HeadlightLeft", "HeadlightRight"), p["lamp_centers"]):
        height, depth = p["lamp_size"][2], p["lamp_size"][1]
        housing = cube(p["lamp_size"], (0, 0, height / 2), red, 0.003)
        bezel = cube((p["lamp_size"][0] - 0.004, 0.003, height - 0.004),
                     (0, depth / 2, height / 2), chrome, 0.001)
        lens = cube(p["lamp_lens_size"], (0, depth / 2 + 0.002, height / 2),
                    "MatLampLens", 0.001)
        parts[name] = finish(name, [housing, bezel, lens], center,
                             (math.radians(p["lamp_rake_deg"]), 0, 0))
    bumper = [cube(p["bumper_size"], material=black, rounding=0.003)]
    for x in (-p["running_light_spacing"], p["running_light_spacing"]):
        bumper.append(cube(p["running_light_size"], (x, p["bumper_size"][1] / 2, 0),
                           "MatLampLens", 0.001))
    parts["Bumper"] = finish("Bumper", bumper, p["bumper_center"])

    # Front surface at local Z=0; three blind instrument recesses and a
    # through-hole for TurboLamp. These are features of DashPanel, not parts.
    dash = cube(p["dash_size"], (0, 0, -p["dash_size"][2] / 2), black, 0.002)
    for x, radius in zip(p["gauge_centers_x"], p["gauge_radii"]):
        cutter = cylinder(radius, 2 * p["gauge_recess_depth"], (x, 0, 0))
        boolean(dash, cutter, "DIFFERENCE")
    boolean(dash, cylinder(p["turbo_lamp_radius"], p["dash_size"][2] * 3,
                           (p["turbo_lamp_center_x"], 0, 0)), "DIFFERENCE")
    parts["DashPanel"] = finish("DashPanel", [dash], p["dash_center"], dash_rot)
    parts["DashPanel"]["TurboLamp_center_local"] = [p["turbo_lamp_center_x"], 0.0, 0.0]
    parts["DashPanel"]["TurboLamp_radius"] = p["turbo_lamp_radius"]
    # Only Screen is textured. Bevel/Boolean interpolation can leave unstable
    # unused UV values; omit those layers instead of exporting irrelevant data.
    # Keep the primitive plane's exact unit-square UVs for the engine texture.
    for name, obj in parts.items():
        if name != "Screen":
            for layer in list(obj.data.uv_layers):
                obj.data.uv_layers.remove(layer)
    bpy.context.view_layer.update()
    return parts, marker


def transform_record(matrix):
    translation, rotation, scale = matrix.decompose()
    return {
        "translation": list(translation),
        "rotation_quaternion_xyzw": [rotation.x, rotation.y, rotation.z, rotation.w],
        "scale": list(scale),
        "matrix_row_major": [list(row) for row in matrix],
    }


def read_glb(path):
    data = path.read_bytes()
    magic, version, length = struct.unpack_from("<4sII", data)
    assert magic == b"glTF" and version == 2 and length == len(data)
    json_length, chunk_type = struct.unpack_from("<I4s", data, 12)
    assert chunk_type == b"JSON"
    return json.loads(data[20:20 + json_length])


def export_and_manifest(parts, marker):
    p = PARAMS
    assert set(parts) == set(PIVOTS)
    assert {obj.name for obj in bpy.context.scene.objects if obj.type == "MESH"} == set(parts)
    assert p["wheel_major_segments"] <= 96
    assert len(parts["Screen"].data.polygons) == 1
    assert parts["Screen"].data.materials[0].name == "MatScreenDark"
    assert (parts["Screen"].matrix_world.translation - marker.matrix_world.translation).length < 1e-7
    vertices = [obj.matrix_world @ vertex.co for obj in parts.values() for vertex in obj.data.vertices]
    lower = [min(v[i] for v in vertices) for i in range(3)]
    upper = [max(v[i] for v in vertices) for i in range(3)]
    assert abs(lower[2]) < 1e-6, "Toy must rest on Z=0"
    assert all(abs(s - 1.0) < 1e-6 for obj in parts.values() for s in obj.scale)

    wheel = parts["SteeringWheel"]
    wheel_extent = [max(v.co[i] for v in wheel.data.vertices) - min(v.co[i] for v in wheel.data.vertices)
                    for i in range(3)]
    assert wheel_extent[2] < min(wheel_extent[:2]) * 0.15, "Wheel must be thin along local Z"
    normal = wheel.matrix_world.to_3x3() @ Vector((0, 0, 1))
    assert abs(normal.z - math.sin(math.radians(p["wheel_tilt_from_vertical_deg"]))) < 1e-6
    assert normal.y < 0, "Wheel must face player"
    lever_extent = [max(v.co[i] for v in parts["GearLever"].data.vertices) for i in range(3)]
    assert abs(lever_extent[1] - p["lever_length"]) < 1e-6

    output = Path(__file__).resolve().parents[1] / "models"
    output.mkdir(parents=True, exist_ok=True)
    glb_path = output / "toy.glb"
    bpy.ops.object.select_all(action="SELECT")
    # Convert WORLD coordinates only. The exporter's automatic Y-up swizzle
    # also swizzles mesh-local positions, turning a local-Z wheel into a
    # local-Y wheel. Preconvert node transforms and disable that swizzle so
    # glTF is Y-up while every part keeps its specified local control axes.
    basis = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
    rest = {name: obj.matrix_world.copy() for name, obj in parts.items()}
    try:
        for name, obj in parts.items():
            obj.matrix_world = basis @ rest[name]
        bpy.context.view_layer.update()
        result = bpy.ops.export_scene.gltf(
            filepath=str(glb_path), export_format="GLB", use_selection=True,
            export_yup=False, export_apply=True, export_materials="EXPORT",
            export_extras=True, export_animations=False, export_cameras=False,
            export_lights=False, export_texcoords=True, export_normals=True,
            export_tangents=False,
        )
    finally:
        for name, obj in parts.items():
            obj.matrix_world = rest[name]
        bpy.context.view_layer.update()
    assert result == {"FINISHED"}, "glTF export failed"
    gltf = read_glb(glb_path)
    nodes = {node["name"]: node for node in gltf["nodes"]}
    assert set(nodes) == set(parts) | {"SCREEN_MARKER"}
    assert all("uri" not in buffer for buffer in gltf.get("buffers", []))
    assert not gltf.get("images") and not gltf.get("textures")

    # Blender Z-up -> glTF Y-up: (x,y,z) -> (x,z,-y). Matrices below describe
    # world-space REST transforms, not node-local transforms. glTF quaternion
    # components use x,y,z,w; row-major matrices are explicitly identified.
    basis = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
    records = []
    exported_world_vertices = []
    for name, obj in parts.items():
        obj.data.calc_loop_triangles()
        primitives = gltf["meshes"][nodes[name]["mesh"]]["primitives"]
        exported_vertices = sum(gltf["accessors"][prim["attributes"]["POSITION"]]["count"] for prim in primitives)
        exported_triangles = sum(gltf["accessors"][prim["indices"]]["count"] // 3 for prim in primitives)
        assert exported_triangles == len(obj.data.loop_triangles)
        # Read the actual exported node TRS and POSITION accessor bounds.
        node = nodes[name]
        q = node.get("rotation", (0, 0, 0, 1))
        exported_matrix = Matrix.LocRotScale(Vector(node.get("translation", (0, 0, 0))),
                                            Quaternion((q[3], *q[:3])),
                                            Vector(node.get("scale", (1, 1, 1))))
        expected_matrix = basis @ obj.matrix_world
        assert all(abs(exported_matrix[i][j] - expected_matrix[i][j]) < 1e-6
                   for i in range(4) for j in range(4)), name
        accessors = [gltf["accessors"][prim["attributes"]["POSITION"]] for prim in primitives]
        local_min = [min(acc["min"][i] for acc in accessors) for i in range(3)]
        local_max = [max(acc["max"][i] for acc in accessors) for i in range(3)]
        if name == "SteeringWheel":
            extent = [hi - lo for lo, hi in zip(local_min, local_max)]
            assert extent[2] < min(extent[:2]) * 0.15, "Export changed wheel spin axis"
        exported_world_vertices.extend(expected_matrix @ v.co for v in obj.data.vertices)
        if name == "Screen":
            # glTF primitives reference materials by index
            screen_mats = {
                gltf["materials"][prim["material"]]["name"] for prim in primitives
            }
            assert screen_mats == {"MatScreenDark"}, screen_mats
            assert all("TEXCOORD_0" in prim["attributes"] for prim in primitives)
        material_names = sorted({slot.material.name for slot in obj.material_slots if slot.material})
        records.append({
            "name": name,
            "pivot_description": PIVOTS[name],
            "pivot_world": list(obj.matrix_world.translation),
            "rest_transform": transform_record(obj.matrix_world),
            "gltf_rest_transform": transform_record(basis @ obj.matrix_world),
            "material_names": material_names,
            "local_bounds": {"min": local_min, "max": local_max},
            "rotation_axis_local": obj.get("rotation_axis_local"),
            "mesh_count": 1,
            "vertex_count": len(obj.data.vertices),
            "polygon_count": len(obj.data.polygons),
            "triangle_count": len(obj.data.loop_triangles),
            "gltf_vertex_count": exported_vertices,
            "gltf_triangle_count": exported_triangles,
            "gltf_primitive_count": len(primitives),
        })
    totals = {key: sum(row[key] for row in records) for key in (
        "mesh_count", "vertex_count", "polygon_count", "triangle_count",
        "gltf_vertex_count", "gltf_triangle_count",
    )}
    assert totals["triangle_count"] <= p["triangle_budget"], totals
    gltf_lower = [min(v[i] for v in exported_world_vertices) for i in range(3)]
    gltf_upper = [max(v[i] for v in exported_world_vertices) for i in range(3)]
    assert abs(gltf_lower[1]) < 1e-6, "Export must rest on glTF Y=0"
    manifest = {
        "schema_version": 2,
        "asset": "toy.glb",
        "units": "meters",
        "coordinates": {"rest_transform": "Blender: X right, Y away from player, Z up",
                        "gltf_rest_transform": "glTF: X right, Y up, Z toward player"},
        "count_notes": "Mesh counts are after modifiers; glTF vertices include normal/material splits.",
        "params": p,
        "parts": records,
        "markers": [{"name": marker.name, "parent": marker.parent.name,
                     "pivot_description": "Screen center; same local axes as ScreenFrame.",
                     "mesh_count": 0, "vertex_count": 0, "material_names": [],
                     "rest_transform": transform_record(marker.matrix_world),
                     "gltf_rest_transform": transform_record(basis @ marker.matrix_world)}],
        "bounds_world": {"min": lower, "max": upper},
        "bounds_gltf_world": {"min": gltf_lower, "max": gltf_upper},
        "assertions": {
            "SteeringWheel": {"passed": True, "disc_plane_local": "XY",
                              "disc_normal_local": "Z", "spin_axis_local": "Z",
                              "local_extent": wheel_extent, "max_thickness_ratio": 0.15,
                              "gltf_accessor_bounds_verified": True},
            "ground_gltf_min_y": gltf_lower[1],
            "screen_has_texcoord_0": True,
            "exported_rest_transforms_verified": True,
            "triangle_budget": p["triangle_budget"],
        },
        "totals": totals,
    }
    (output / "toy.manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("Created parts (mesh vertices / exported vertices / triangles):")
    for row in records:
        print(f"  {row['name']}: {row['vertex_count']} / {row['gltf_vertex_count']} / {row['triangle_count']}")
    print(f"Summary: {len(parts)} parts + SCREEN_MARKER; {totals['vertex_count']} mesh vertices, "
          f"{totals['gltf_vertex_count']} exported vertices, {totals['triangle_count']} triangles; "
          "assets/models/toy.glb and assets/models/toy.manifest.json")


def main():
    # No sound is used. Avoid contacting desktop audio services in headless CI.
    bpy.context.preferences.system.audio_device = "None"
    # Clear even unused factory materials so names are stable on repeated runs.
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    for material in list(bpy.data.materials):
        bpy.data.materials.remove(material)
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.scene.unit_settings.scale_length = 1.0
    create_materials()
    parts, marker = build_parts()
    export_and_manifest(parts, marker)


if __name__ == "__main__":
    exit_code = 0
    try:
        main()
    except Exception:
        # Blender can otherwise return 0 for Python errors without the caller's
        # --python-exit-code flag. Make the requested command fail truthfully.
        import traceback
        traceback.print_exc()
        exit_code = 1
    if bpy.app.background:
        # All export/file writes above are synchronous and closed. Blender's
        # PulseAudio teardown can hang in a sandbox even with audio disabled;
        # this standalone batch generator needs no further Blender cleanup.
        # Preserve failures and flush the summary before exiting the process.
        sys.stdout.flush()
        sys.stderr.flush()
        os._exit(exit_code)
    if exit_code:
        raise SystemExit(exit_code)
