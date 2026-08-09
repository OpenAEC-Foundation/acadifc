//! Lifting an ACIS document into the kernel and lowering it back.
//!
//! The property that matters is not that a body survives: it is that a body
//! *nobody edited* comes back byte for byte, so opening and saving a drawing
//! is not a slow way of losing everything the kernel does not model.

use acadifc::acis::{lift, lower, pending};
use acadifc::kernel::brep::{Provenance, Surface};
use cadcodec::entities::acis::types::{SatDocument, SatPointer, SatRecord, SatToken};

fn record(kind: &str, tokens: Vec<SatToken>) -> SatRecord {
    SatRecord {
        index: -1,
        entity_type: kind.to_string(),
        sub_type: None,
        attribute: SatPointer::new(-1),
        subtype_id: -1,
        tokens,
        raw_text: None,
    }
}

fn pointer(index: i32) -> SatToken {
    SatToken::Pointer(SatPointer::new(index))
}

fn at(x: f64, y: f64, z: f64) -> SatToken {
    SatToken::Position(x, y, z)
}

/// A body with one lump, one shell, one planar face bounded by one loop of
/// four coedges — the smallest thing that exercises the whole walk.
fn one_face_document() -> SatDocument {
    let mut document = SatDocument::new();
    // 0 body → 1 lump → 2 shell → 3 face
    document.add_record(record("body", vec![pointer(-1), pointer(1), pointer(-1), pointer(-1)]));
    document.add_record(record("lump", vec![pointer(-1), pointer(-1), pointer(2), pointer(0)]));
    document.add_record(record(
        "shell",
        vec![pointer(-1), pointer(-1), pointer(-1), pointer(3), pointer(-1), pointer(1)],
    ));
    // face: attribute, next, loop, shell, subshell, surface, sense
    document.add_record(record(
        "face",
        vec![
            pointer(-1),
            pointer(-1),
            pointer(4),
            pointer(2),
            pointer(-1),
            pointer(5),
            SatToken::Ident("forward".to_string()),
            SatToken::Ident("single".to_string()),
        ],
    ));
    // loop: attribute, next, coedge, face
    document.add_record(record(
        "loop",
        vec![pointer(-1), pointer(-1), pointer(6), pointer(3)],
    ));
    // plane-surface: attribute, root, normal, u
    document.add_record(record(
        "plane-surface",
        vec![pointer(-1), at(0.0, 0.0, 0.0), at(0.0, 0.0, 1.0), at(1.0, 0.0, 0.0)],
    ));
    // Four coedges in a ring, 6..9, each on its own edge 10..13.
    for step in 0..4 {
        let next = 6 + (step + 1) % 4;
        let previous = 6 + (step + 3) % 4;
        document.add_record(record(
            "coedge",
            vec![
                pointer(-1),
                pointer(next),
                pointer(previous),
                pointer(-1),
                pointer(10 + step),
                SatToken::Ident("forward".to_string()),
                pointer(4),
                pointer(-1),
            ],
        ));
    }
    // Four edges 10..13, each between two of the vertices 14..17 on a line.
    for step in 0..4 {
        document.add_record(record(
            "edge",
            vec![
                pointer(-1),
                pointer(14 + step),
                SatToken::Float(0.0),
                pointer(14 + (step + 1) % 4),
                SatToken::Float(1.0),
                pointer(6 + step),
                pointer(18),
                SatToken::Ident("forward".to_string()),
            ],
        ));
    }
    // Four vertices 14..17, each naming a point 19..22.
    for step in 0..4 {
        document.add_record(record(
            "vertex",
            vec![pointer(-1), pointer(10 + step), pointer(19 + step)],
        ));
    }
    // 18: the straight curve every edge runs along.
    document.add_record(record(
        "straight-curve",
        vec![pointer(-1), at(0.0, 0.0, 0.0), at(1.0, 0.0, 0.0)],
    ));
    // 19..22: the corners.
    for corner in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)] {
        document.add_record(record(
            "point",
            vec![pointer(-1), at(corner.0, corner.1, 0.0)],
        ));
    }
    document
}

#[test]
fn a_body_lifts_with_its_topology_intact() {
    let document = one_face_document();
    let (bodies, loss) = lift(&document);
    assert_eq!(bodies.len(), 1, "one body record, one body");
    assert!(loss.is_empty(), "{loss:?}");
    let body = &bodies[0];
    assert_eq!(body.lumps.len(), 1);
    assert_eq!(body.shells.len(), 1);
    assert_eq!(body.faces.len(), 1);
    assert_eq!(body.loops.len(), 1);
    assert_eq!(body.coedges.len(), 4, "the ring closed rather than running on");
    assert_eq!(body.edges.len(), 4);
    assert_eq!(body.vertices.len(), 4);
}

#[test]
fn the_surface_is_the_one_the_file_named() {
    let (bodies, _) = lift(&one_face_document());
    let body = &bodies[0];
    let (_, face) = body.faces.iter().next().unwrap();
    let Surface::Plane(plane) = body.surfaces.get(face.surface).unwrap() else {
        panic!("the file says plane-surface");
    };
    // The frame comes from the file's own u direction rather than being
    // derived, which is why the kernel's Plane takes axes.
    assert_eq!(plane.origin, [0.0, 0.0, 0.0]);
    assert_eq!(plane.x_axis, [1.0, 0.0, 0.0]);
    assert_eq!(plane.normal(), Some([0.0, 0.0, 1.0]));
}

#[test]
fn a_record_shared_by_several_nodes_becomes_one_node() {
    // All four edges name the same straight-curve. Lifting it four times
    // would leave four curves that a later edit would have to keep in step.
    let (bodies, _) = lift(&one_face_document());
    assert_eq!(bodies[0].curves.len(), 1);
}

#[test]
fn every_lifted_node_remembers_where_it_came_from() {
    let (bodies, _) = lift(&one_face_document());
    let body = &bodies[0];
    for (_, face) in body.faces.iter() {
        assert!(matches!(face.provenance, Provenance::Clean(_)));
    }
    for (_, edge) in body.edges.iter() {
        assert!(matches!(edge.provenance, Provenance::Clean(_)));
    }
    for (_, vertex) in body.vertices.iter() {
        assert!(matches!(vertex.provenance, Provenance::Clean(_)));
    }
}

#[test]
fn an_untouched_body_lowers_back_byte_for_byte() {
    // The whole point. Opening a drawing and saving it must not rewrite the
    // solids through whatever subset of ACIS this kernel models.
    let mut document = one_face_document();
    let before = document.to_sat_string();
    let (bodies, _) = lift(&document);
    assert_eq!(pending(&bodies[0]), 0, "nothing was touched");
    let written = lower(&bodies[0], &mut document).expect("an untouched body lowers");
    assert_eq!(written, 0, "no record needed rewriting");
    assert_eq!(document.to_sat_string(), before);
}

#[test]
fn an_edit_rewrites_only_what_it_touched() {
    let mut document = one_face_document();
    let (mut bodies, _) = lift(&document);
    let body = &mut bodies[0];
    let corner = body.vertices.keys().next().unwrap();
    body.vertices.get_mut(corner).unwrap().point = [1.0, 2.0, 3.0];
    body.soil_vertex(corner);

    // A corner of a quadrilateral belongs to two edges, which bound one face.
    assert_eq!(pending(body), 1 + 2 + 1, "one vertex, two edges, one face");
    let written = lower(body, &mut document).expect("a moved corner lowers");
    assert_eq!(written, 4);
    // And the topology is still intact: the records the geometry hangs off
    // were not the ones written to.
    let (relifted, loss) = lift(&document);
    assert!(loss.is_empty(), "{loss:?}");
    assert_eq!(relifted[0].vertices.len(), 4);
    assert_eq!(relifted[0].edges.len(), 4);
}

#[test]
fn a_moved_corner_comes_back_where_it_was_put() {
    let mut document = one_face_document();
    let (mut bodies, _) = lift(&document);
    let body = &mut bodies[0];
    let corner = body.vertices.keys().next().unwrap();
    body.vertices.get_mut(corner).unwrap().point = [1.0, 2.0, 3.0];
    body.soil_vertex(corner);
    lower(body, &mut document).unwrap();

    let text = document.to_sat_string();
    let back = SatDocument::parse(&text).expect("reparses");
    let (again, loss) = lift(&back);
    assert!(loss.is_empty(), "{loss:?}");
    let moved = again[0]
        .vertices
        .iter()
        .any(|(_, v)| v.point == [1.0, 2.0, 3.0]);
    assert!(moved, "the edit did not survive the round trip");
}

#[test]
fn a_surface_the_kernel_does_not_know_is_reported_rather_than_dropped() {
    let mut document = one_face_document();
    // Turn the plane into something with no kernel case.
    document.record_mut(5).unwrap().entity_type = "spline-surface".to_string();
    let (bodies, loss) = lift(&document);
    assert!(!loss.surfaces.is_empty(), "the loss is named");
    // The face could not be built without a surface, so the body has none —
    // but the caller was told, rather than being handed a body that looks
    // whole.
    assert!(bodies.is_empty() || bodies[0].faces.is_empty());
}

#[test]
fn a_straight_edge_is_placed_by_its_vertices_not_by_its_record() {
    // The fault this guards against, seen in the wild: a straight edge whose
    // stored parameter pair spans the right *length* from the wrong place, so
    // the curve runs alongside the face it is supposed to bound rather than
    // along it. The loop then fails to close in parameter space, the face is
    // dropped, and the solid comes out with a hole in it — from a record that
    // reads as perfectly well formed.
    //
    // A line's parameter is a distance along an infinite curve, so its two
    // vertices fix it and nothing else can. They are believed.
    let mut document = one_face_document();
    let edge = document
        .records
        .iter_mut()
        .find(|record| record.entity_type == "edge")
        .expect("an edge to spoil");
    // The first edge runs from (0, 0) to (10, 0) along the x axis, so 0..10 is
    // the truth. Ten units of the line starting ten further along is the same
    // length in the wrong place.
    edge.tokens[2] = SatToken::Float(10.0);
    edge.tokens[4] = SatToken::Float(20.0);

    let (bodies, _) = lift(&document);
    let body = bodies.first().expect("a body");
    // The spoiled edge runs between (0, 0) and (10, 0), so on a line through
    // the origin along x its parameters are 0 and 10 — whatever the record
    // claimed. Reading 10 and 20 back would put it ten units past the corner.
    let spoiled = body
        .edges
        .iter()
        .find(|(key, _)| {
            body.edge_endpoints(*key)
                .is_some_and(|(start, end)| start[0] == 0.0 && end[0] == 10.0)
        })
        .map(|(_, node)| (node.start_parameter, node.end_parameter))
        .expect("the edge between the first two corners");
    assert_eq!(spoiled, (0.0, 10.0), "the record was believed over the vertices");
}
