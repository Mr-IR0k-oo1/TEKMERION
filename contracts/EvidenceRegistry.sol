// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title Evidence Registry
/// @notice Stores evidence hashes on the blockchain
contract EvidenceRegistry {
    /// @dev Evidence structure
    struct Evidence {
        bytes32 dataHash;
        uint256 timestamp;
        address submitter;
    }

    /// @dev Mapping of evidence hashes to Evidence structs
    mapping(bytes32 => Evidence) public records;

    /// @dev Event emitted when evidence is registered
    event EvidenceRegistered(bytes32 indexed hash, address indexed submitter);

    /// @notice Register evidence
    /// @param hash SHA-256 hash of the evidence
    /// @dev Reverts if the hash already exists
    function registerEvidence(bytes32 hash) external {
        require(records[hash].dataHash == bytes32(0), "Evidence already registered");

        records[hash] = Evidence({
            dataHash: hash,
            timestamp: block.timestamp,
            submitter: msg.sender
        });

        emit EvidenceRegistered(hash, msg.sender);
    }

    /// @notice Verify evidence
    /// @param hash SHA-256 hash of the evidence
    /// @return bool True if the evidence exists
    function verifyEvidence(bytes32 hash) external view returns (bool) {
        return records[hash].dataHash != bytes32(0);
    }
}