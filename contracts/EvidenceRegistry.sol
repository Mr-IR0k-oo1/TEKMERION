// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Evidence Registry
/// @notice Immutably stores cryptographic evidence roots and candidate image hashes on-chain.
contract EvidenceRegistry {
    /// @dev Tamper-evident record anchored on-chain
    struct Evidence {
        bytes32 rootHash;
        bytes32 imageHash;
        uint256 timestamp;
        address submitter;
    }

    /// @dev Mapping from Merkle root hash to Evidence record
    mapping(bytes32 => Evidence) public records;

    /// @dev Emitted whenever a new evidence record is registered
    event EvidenceRegistered(
        bytes32 indexed rootHash,
        bytes32 indexed imageHash,
        uint256 timestamp,
        address indexed submitter
    );

    /// @notice Register a new evidence bundle on-chain
    /// @param rootHash SHA-256 Merkle root hash of the canonical evidence tree
    /// @param imageHash SHA-256 hash of the matched candidate image
    function registerEvidence(bytes32 rootHash, bytes32 imageHash) external {
        require(rootHash != bytes32(0), "Invalid root hash");
        require(records[rootHash].rootHash == bytes32(0), "Evidence root already registered");

        records[rootHash] = Evidence({
            rootHash: rootHash,
            imageHash: imageHash,
            timestamp: block.timestamp,
            submitter: msg.sender
        });

        emit EvidenceRegistered(rootHash, imageHash, block.timestamp, msg.sender);
    }

    /// @notice Backwards-compatible single-hash registration
    /// @param hash SHA-256 hash of the evidence
    function registerEvidence(bytes32 hash) external {
        registerEvidence(hash, bytes32(0));
    }

    /// @notice Retrieve full evidence record by root hash
    /// @param rootHash SHA-256 Merkle root hash
    /// @return root Stored root hash
    /// @return image Stored image hash
    /// @return timestamp Block timestamp when registered
    /// @return submitter Wallet address of submitter
    function getEvidence(bytes32 rootHash) external view returns (
        bytes32 root,
        bytes32 image,
        uint256 timestamp,
        address submitter
    ) {
        Evidence memory ev = records[rootHash];
        return (ev.rootHash, ev.imageHash, ev.timestamp, ev.submitter);
    }

    /// @notice Verify whether a given evidence root exists on-chain
    /// @param rootHash SHA-256 Merkle root hash
    /// @return bool True if registered
    function verifyEvidence(bytes32 rootHash) external view returns (bool) {
        return records[rootHash].rootHash != bytes32(0);
    }
}