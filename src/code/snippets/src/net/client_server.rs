use fyrox::fxhash::FxHashMap;
use fyrox::scene::node::Node;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        info,
        net::{NetListener, NetStream},
        pool::Handle,
        reflect::prelude::*,
        some_or_return,
        visitor::prelude::*,
    },
    graph::SceneGraph,
    plugin::{Plugin, PluginContext},
    scene::{base::SceneNodeId, Scene},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Formatter},
    path::Path,
    path::PathBuf,
};

// ANCHOR: messages
// Server messages are meant to be sent to clients.
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    LoadLevel { path: PathBuf },
    Sync { entity_states: Vec<NodeState> },
}

#[derive(Serialize, Deserialize, Debug)]
// Client messages are meant to be sent to a server.
pub enum ClientMessage {
    PlayerInput { left: bool, right: bool },
}
// ANCHOR_END: messages

// ANCHOR: client_server
// Implements listen server.
#[derive(Default, Reflect, Debug)]
pub struct Game {
    scene: Handle<Scene>,
    server: Option<Server>,
    client: Option<Client>,
}

impl Plugin for Game {
    fn init(&mut self, scene_path: Option<&str>, context: PluginContext) {
        self.server = Some(Server::new());
        self.client = Some(Client::connect(Server::ADDRESS));
    }

    // ANCHOR: update_loop
    fn update(&mut self, context: &mut PluginContext) {
        if let Some(server) = self.server.as_mut() {
            server.accept_connections();
            server.read_messages();
        }
        if let Some(client) = self.client.as_mut() {
            client.read_messages();
        }
    }
    // ANCHOR_END: update_loop

    // ANCHOR: disable_physics
    fn on_scene_loaded(
        &mut self,
        path: &Path,
        scene: Handle<Scene>,
        data: &[u8],
        context: &mut PluginContext,
    ) {
        self.scene = scene;

        if self.server.is_none() {
            context.scenes[scene]
                .graph
                .physics
                .enabled
                .set_value_and_mark_modified(false);
        }
    }
    // ANCHOR_END: disable_physics
}

#[derive(Reflect)]
pub struct Server {
    #[reflect(hidden)]
    listener: NetListener,
    #[reflect(hidden)]
    connections: Vec<NetStream>,
    #[reflect(hidden)]
    prev_node_states: FxHashMap<Handle<Node>, NodeState>,
}

impl Server {
    const ADDRESS: &'static str = "127.0.0.1:10000";

    pub fn new() -> Self {
        Self {
            listener: NetListener::bind(Self::ADDRESS).unwrap(),
            connections: Default::default(),
            prev_node_states: Default::default(),
        }
    }

    pub fn accept_connections(&mut self) {
        self.connections.extend(self.listener.accept_connections())
    }

    pub fn read_messages(&mut self) {
        for connection in self.connections.iter_mut() {
            connection
                .process_input::<ClientMessage>(|msg| info!("Received client message: {msg:?}"));
        }
    }

    pub fn send_message_to_clients(&mut self, message: ServerMessage) {
        for connection in self.connections.iter_mut() {
            connection.send_message(&message).unwrap();
        }
    }
}

#[derive(Reflect)]
pub struct Client {
    #[reflect(hidden)]
    connection: NetStream,
}

impl Client {
    pub fn connect(address: &str) -> Self {
        Self {
            connection: NetStream::connect(address).unwrap(),
        }
    }

    pub fn read_messages(&mut self) {
        self.connection
            .process_input::<ServerMessage>(|msg| info!("Received server message: {msg:?}"));
    }

    pub fn send_message_to_server(&mut self, message: ClientMessage) {
        self.connection.send_message(&message).unwrap();
    }
}

impl Visit for Game {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // This must be implemented only for hot-reloading support. It is up to you to maintain
        // client-server connections during hot-reloading, and it is out of the scope of this example.
        Ok(())
    }
}
// ANCHOR_END: client_server

// ANCHOR: send_test_messages
impl Game {
    fn send_test_messages(&mut self) {
        // Send the server message to the clients.
        if let Some(server) = self.server.as_mut() {
            // Force clients to load a level.
            server.send_message_to_clients(ServerMessage::LoadLevel {
                path: PathBuf::from("data/scenes/scene.rgs"),
            })
        }
        // Send the client message.
        if let Some(client) = self.client.as_mut() {
            client.send_message_to_server(ClientMessage::PlayerInput {
                // Player's moving left.
                left: true,
                right: false,
            })
        }
    }
}
// ANCHOR_END: send_test_messages

// ANCHOR: simple_syncing
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NodeState {
    pub node: SceneNodeId,
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
}

impl Server {
    pub fn sync(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        let scene = some_or_return!(ctx.scenes.try_get(scene));
        let mut entity_states = Vec::with_capacity(scene.graph.capacity() as usize);
        for (handle, node) in scene.graph.pair_iter() {
            entity_states.push(NodeState {
                node: node.instance_id(),
                position: **node.local_transform().position(),
                rotation: **node.local_transform().rotation(),
            });
        }
        self.send_message_to_clients(ServerMessage::Sync { entity_states });
    }
}
// ANCHOR_END: simple_syncing

// ANCHOR: syncing_with_delta_compression
impl Server {
    pub fn sync_with_delta_compression(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        let scene = some_or_return!(ctx.scenes.try_get(scene));
        let mut entity_states = Vec::with_capacity(scene.graph.capacity() as usize);
        for (handle, node) in scene.graph.pair_iter() {
            let current_state = NodeState {
                node: node.instance_id(),
                position: **node.local_transform().position(),
                rotation: **node.local_transform().rotation(),
            };

            // Simple delta compression.
            let prev_state = self
                .prev_node_states
                .entry(handle)
                .or_insert(current_state.clone());

            if *prev_state != current_state {
                entity_states.push(current_state.clone());
                *prev_state = current_state;
            }
        }

        self.send_message_to_clients(ServerMessage::Sync { entity_states });
    }
}
// ANCHOR_END: syncing_with_delta_compression

impl Debug for Server {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server")
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client")
    }
}
